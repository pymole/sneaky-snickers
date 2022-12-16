use std::str::FromStr;
use std::sync::{Mutex, Arc, Condvar};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::thread;

use mongodb::bson::oid::ObjectId;
use mongodb::bson::Bson;
use mongodb::sync::Client;
use rand::{thread_rng, Rng};

use crate::features::collector::{collect_examples, Example, ExamplesHandler};
use crate::features::composite::CompositeExamples;
use crate::features::examples::ExamplesContext;
use crate::game_log::{load_game_log, rewind};

enum LoaderCommand {
    LoadGameLog,
    Close,
}

pub struct Loader {
    client: Client,
    game_log_ids: Vec<ObjectId>,
    loader_receiver: Receiver<LoaderCommand>,
    examples: Arc<(Mutex<Vec<Example>>, Condvar)>,
    loader_is_empty: Arc<Mutex<bool>>,
    composite_examples: CompositeExamples,
}

impl Loader {
    fn new(
        client: Client,
        game_log_ids: Vec<String>,
        feature_set_tags: Vec<String>,
        loader_receiver: Receiver<LoaderCommand>,
        examples: Arc<(Mutex<Vec<Example>>, Condvar)>,
        loader_is_empty: Arc<Mutex<bool>>,
    ) -> Self {
        let game_log_ids: Vec<ObjectId> = game_log_ids
            .into_iter()
            .map(|x| ObjectId::from_str(x.as_str()).unwrap())
            .collect();
        
        let feature_set_tags = feature_set_tags.into_iter().map(|x| x.to_string()).collect();

        // WARN: You can't use context and composite examples before you set some actual board data.
        let context = ExamplesContext::new();
        let composite_examples = CompositeExamples::new(feature_set_tags, context);

        Loader {
            client,
            game_log_ids,
            loader_receiver,
            examples,
            loader_is_empty,
            composite_examples,
        }
    }
    
    fn start(mut self) {
        loop {
            let command = self.loader_receiver.recv().unwrap();
            match command {
                LoaderCommand::LoadGameLog => {
                    let examples_option = self.next_examples();
                    let (examples_mutex, examples_waiting) = self.examples.as_ref();
                    // Lock examples even if no game logs left or error is occured
                    // because mixer mustn't get notification before it's waiting on condvar.
                    let mut examples = examples_mutex.lock().unwrap();

                    if let Some(loaded_examples) = examples_option {
                        examples.extend(loaded_examples);
                        examples_waiting.notify_one();
                        // println!("Game log");
                    } else {
                        // No game logs or error in Loader and Mixer is waiting - release Mixer
                        *self.loader_is_empty.lock().unwrap() = true;
                        examples_waiting.notify_one();
                        // println!("Error or no game logs");
                        break;
                    }
                },
                LoaderCommand::Close => {
                    // println!("Loader is closing");
                    break;
                }
            }
        }
    }

    fn next_examples(&mut self) -> Option<Vec<Example>> {
        if self.game_log_ids.is_empty() {
            return None;
        }

        let id = self.game_log_ids.pop().unwrap();
        let option = load_game_log(&self.client, Bson::ObjectId(id));
        if let Some(game_log) = option {
            let (_, mut boards) = rewind(&game_log);
     
            let terminal_board = boards.pop().unwrap();
            self.composite_examples.examples_collector
                .borrow_mut()
                .context
                .set_actual_rewards(&terminal_board);

            let mut examples = Vec::new();
            for board in boards {
                // TODO: This ugly
                {
                    let mut examples_collector = self.composite_examples.examples_collector.borrow_mut();
                    examples_collector.context.set_board(&board);
                    examples_collector.refresh_collectors();    
                }
                
                collect_examples(&board, &mut self.composite_examples);                
                examples.extend(self.composite_examples.pop_examples());
            }

            Some(examples)
        } else {
            None
        }
    }

}

enum MixerCommand {
    NeedBatch,
    Close,
}

pub struct Mixer {
    batch_size: usize,
    examples: Arc<(Mutex<Vec<Example>>, Condvar)>,
    mixer_size: usize,
    loader_sender: Sender<LoaderCommand>,
    loader_is_empty: Arc<Mutex<bool>>,
    mixer_receiver: Receiver<MixerCommand>,
    batch_sender: Sender<BatchResult>,
    random_batch: bool,
}

impl Mixer {
    fn new(
        batch_size: usize,
        mixer_size: usize,
        loader_sender: Sender<LoaderCommand>,
        batch_sender: Sender<BatchResult>,
        mixer_receiver: Receiver<MixerCommand>,
        loader_is_empty: Arc<Mutex<bool>>,
        random_batch: bool,
    ) -> Self {
        assert!(mixer_size >= batch_size);
        let examples = Arc::new((Mutex::new(Vec::new()), Condvar::new()));

        // Load first game log
        loader_sender.send(LoaderCommand::LoadGameLog).unwrap();

        Mixer {
            batch_size,
            loader_sender,
            examples,
            mixer_size,
            batch_sender,
            mixer_receiver,
            loader_is_empty,
            random_batch,
        }
    }

    fn start(self, prefetch_batches: usize) {
        // TODO: maybe in dataloader before thread spawn?
        for _ in 0..prefetch_batches {
            let result = self.send_batch();
            if result.is_err() {
                return;
            }
        }

        loop {
            // TODO: break on unwrap error 
            let command = self.mixer_receiver.recv().unwrap();

            match command {
                MixerCommand::NeedBatch => {
                    let result = self.send_batch();
                    if result.is_err() {
                        break;
                    }
                }
                MixerCommand::Close => {
                    self.loader_sender.send(LoaderCommand::Close);
                    break;
                }
            }
        }
    }

    fn send_batch(&self) -> Result<(), ()> {
        let (examples_mutex, examples_waiting) = self.examples.as_ref();
        let mut examples = examples_mutex.lock().unwrap();
        
        if examples.len() < self.batch_size {
            if self.is_loader_empty() {
                self.batch_sender.send(BatchResult::Empty).unwrap();
                return Err(())
            }

            // println!("Waiting");
            examples = examples_waiting.wait(examples).unwrap();
            // println!("Released");

            if self.is_loader_empty() {
                self.batch_sender.send(BatchResult::Empty).unwrap();
                return Err(())
            }

            if examples.len() < self.batch_size {
                // TODO: Case when loader got less examples than batch needed
                //  Potentialy control load flow in Loader, not in mixer.
                self.loader_sender.send(LoaderCommand::Close).unwrap();
                self.batch_sender.send(BatchResult::Empty).unwrap();
                return Err(());
            }
        }

        let batch = if self.random_batch {
            // Construct batch of random examples
            let mut batch = Vec::new();
            let rng = &mut thread_rng();
            for _ in 0..self.batch_size {
                let i = rng.gen_range(0..examples.len());
                let example = examples.swap_remove(i);
                batch.push(example);
            }
            batch
        } else {
            let examples_count = examples.len();
            // Or pick as it stored in examples
            let batch = examples.drain(examples_count - self.batch_size..).collect();
            batch
        };
        
        self.batch_sender.send(BatchResult::Batch(batch)).unwrap();

        if examples.len() < self.mixer_size && !self.is_loader_empty() {
            self.loader_sender.send(LoaderCommand::LoadGameLog).unwrap();
        }

        Ok(())
    }

    fn is_loader_empty(&self) -> bool {
        *self.loader_is_empty.lock().unwrap()
    }
}

enum BatchResult {
    Batch(Vec<Example>),
    Empty,
}

pub struct DataLoader {
    mixer_sender: Sender<MixerCommand>,
    batch_receiver: Receiver<BatchResult>,
}

impl DataLoader {
    pub fn new(
        mongo_uri: String,
        batch_size: usize,
        prefetch_batches: usize,
        mixer_size: usize,
        game_log_ids: Vec<String>,
        feature_set_tags: Vec<String>,
        random_batch: bool,
    ) -> Self {
        assert!(!game_log_ids.is_empty());
        assert!(!feature_set_tags.is_empty());

        let client = Client::with_uri_str(mongo_uri).unwrap();
        let (loader_sender, loader_receiver) = channel();
        let (batch_sender, batch_receiver) = channel();
        let (mixer_sender, mixer_receiver) = channel();
        let loader_is_empty = Arc::new(Mutex::new(false));

        let mixer = Mixer::new(
            batch_size,
            mixer_size,
            loader_sender,
            batch_sender,
            mixer_receiver,
            loader_is_empty.clone(),
            random_batch,
        );

        let mixer_examples = mixer.examples.clone();

        thread::spawn(move || {
            let loader = Loader::new(
                client,
                game_log_ids,
                feature_set_tags,
                loader_receiver,
                mixer_examples,
                loader_is_empty,
            );    
            loader.start()
        });
        thread::spawn(move || mixer.start(prefetch_batches));

        DataLoader {
            mixer_sender,
            batch_receiver,
        }
    }
}

impl Drop for DataLoader {
    fn drop(&mut self) {
        self.mixer_sender.send(MixerCommand::Close);
    }
}

impl Iterator for DataLoader {
    type Item = Vec<Example>;

    fn next(&mut self) -> Option<Self::Item> {
        let batch_result = self.batch_receiver.recv().unwrap();
        let send_res = self.mixer_sender.send(MixerCommand::NeedBatch);
        if send_res.is_err() {
            return None;
        }

        if let BatchResult::Batch(batch) = batch_result {
            Some(batch)
        } else {
            None
        }
    }
}

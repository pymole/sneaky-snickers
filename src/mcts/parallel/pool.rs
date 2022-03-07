use std::sync::mpsc::channel;
use std::thread::{spawn, JoinHandle};
use std::sync::mpsc::{Sender, Receiver};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

pub struct Task<R> {
    pub fn_box: Box<dyn FnOnce() -> R + Send + 'static>,
}

enum Command<R> {
    Execute(Task<R>),
    Stop
}


pub struct Pool<R>
{
    workers: Vec<JoinHandle<()>>,
    tasks_count: usize,
    command_sender: Sender<Command<R>>,
    result_receiver: Receiver<R>,
    wait_time: Arc<Mutex<Duration>>,
}

impl<R> Pool<R>
where
    R: Send + 'static
{
    pub fn new(workers: usize) -> Pool<R> {
        assert!(workers > 0);

        let (command_sender, command_receiver) = channel();
        let (result_sender, result_receiver) = channel();

        let command_receiver: Arc<Mutex<Receiver<Command<R>>>> = Arc::new(Mutex::new(command_receiver));
        let result_sender: Arc<Mutex<Sender<R>>> = Arc::new(Mutex::new(result_sender));

        let wait_time = Arc::new(Mutex::new(Duration::new(0, 0)));
        
        let workers = (0..workers).into_iter()
            .map(|_| {
                let command_receiver_ = command_receiver.clone();
                let result_sender_ = result_sender.clone();
                let wait_time_ = wait_time.clone();
                
                spawn(move || {
                    loop {
                        let start = Instant::now();
                        let command = command_receiver_.lock().unwrap().recv().unwrap();
                        *wait_time_.lock().unwrap() += Instant::now() - start;
                        
                        match command {
                            Command::Execute(task) => {
                                let result = (task.fn_box)();
                                result_sender_.lock().unwrap().send(result).unwrap();
                            }
                            Command::Stop => break
                        }
                    }
                })
            })
            .collect();
        
        Pool {
            workers,
            tasks_count: 0,
            command_sender,
            result_receiver,
            wait_time,
        }
    }

    pub fn add_task(&mut self, task: Task<R>) {
        self.command_sender.send(Command::Execute(task)).unwrap();
        self.tasks_count += 1;
    }

    pub fn wait_result(&mut self) -> R {
        let result = self.result_receiver.recv().unwrap();
        self.tasks_count -= 1;
        result
    }

    pub fn tasks_count(&self) -> usize {
        self.tasks_count
    }

    pub fn wait_time(&self) -> Duration {
        self.wait_time.lock().unwrap().clone()
    }

    pub fn shutdown(&self) {
        for _ in 0..self.workers.len() {
            self.command_sender.send(Command::Stop).unwrap();
        }
    }
}

use std::sync::mpsc::channel;
use std::thread::{spawn, JoinHandle};
use std::sync::mpsc::{Sender, Receiver};
use std::sync::{Arc, Mutex};

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

        let workers = (0..workers).into_iter()
            .map(|_| {
                let command_receiver_ = command_receiver.clone();
                let result_sender_ = result_sender.clone();
                
                spawn(move || {
                    loop {
                        let command = command_receiver_.lock().unwrap().recv().unwrap();
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
        }
    }

    pub fn add_task(&mut self, task: Task<R>) {
        self.command_sender.send(Command::Execute(task)).unwrap();
    }

    pub fn wait_result(&mut self) -> R {
        let result = self.result_receiver.recv().unwrap();
        self.tasks_count -= 1;
        result
    }

    pub fn tasks_count(&self) -> usize {
        self.tasks_count
    }

    pub fn shutdown(&self) {
        for _ in 0..self.workers.len() {
            self.command_sender.send(Command::Stop).unwrap();
        }
    }
}

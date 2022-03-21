use std::thread::{spawn, JoinHandle};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use crossbeam_channel::{unbounded, Sender, Receiver};

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
    workers_wait_time: Arc<Mutex<Duration>>,
    result_wait_time: Duration,
}

impl<R> Pool<R>
where
    R: Send + 'static
{
    pub fn new(workers: usize) -> Pool<R> {
        assert!(workers > 0);

        let (command_sender, command_receiver) = unbounded();
        let (result_sender, result_receiver) = unbounded();

        let workers_wait_time = Arc::new(Mutex::new(Duration::new(0, 0)));
        
        let workers = (0..workers).into_iter()
            .map(|_| {
                let command_receiver_ = command_receiver.clone();
                let result_sender_ = result_sender.clone();
                let workers_wait_time_ = workers_wait_time.clone();
                
                spawn(move || {
                    loop {
                        let start = Instant::now();
                        let command = command_receiver_.recv().unwrap();
                        *workers_wait_time_.lock().unwrap() += Instant::now() - start;
                        
                        match command {
                            Command::Execute(task) => {
                                let result = (task.fn_box)();
                                if let Err(_) = result_sender_.send(result) {
                                    break
                                }
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
            workers_wait_time,
            result_wait_time: Duration::from_secs(0),
        }
    }

    pub fn add_task(&mut self, task: Task<R>) {
        self.command_sender.send(Command::Execute(task)).unwrap();
        self.tasks_count += 1;
    }

    pub fn wait_result(&mut self) -> R {
        let start = Instant::now();
        let result = self.result_receiver.recv().unwrap();
        self.result_wait_time += Instant::now() - start;

        self.tasks_count -= 1;
        result
    }

    pub fn tasks_count(&self) -> usize {
        self.tasks_count
    }

    pub fn workers_wait_time(&self) -> Duration {
        self.workers_wait_time.lock().unwrap().clone()
    }

    pub fn result_wait_time(&self) -> Duration {
        self.result_wait_time
    }

    pub fn shutdown(&self) {
        for _ in 0..self.workers.len() {
            self.command_sender.send(Command::Stop).unwrap();
        }
    }
}

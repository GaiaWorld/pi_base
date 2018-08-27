use std::sync::Arc;
use std::boxed::FnBox;
use std::sync::atomic::{AtomicUsize, Ordering};

use npnc::bounded::mpmc::{channel as npnc_channel, Producer};

use pi_lib::atom::Atom;

use task::TaskType;
use future::FutTask;

/*
* 未来任务池
*/
#[derive(Debug)]
pub struct FutTaskPool {
    counter:    AtomicUsize,                                //未来任务计数器
    executor:   fn(TaskType, u64, Box<FnBox()>, Atom),      //未来任务执行器
}

impl FutTaskPool {
    //构建一个未来任务池
    pub fn new(executor: fn(TaskType, u64, Box<FnBox()>, Atom)) -> Self {
        FutTaskPool {
            counter: AtomicUsize::new(0),
            executor: executor,
        }
    }

    //获取当前未来任务计数
    pub fn counte(&self) -> usize {
        self.counter.load(Ordering::Relaxed)
    }

    //分派一个未来任务
    pub fn spawn<T, E>(&self, callback: Box<FnBox(fn(TaskType, u64, Box<FnBox()>, Atom), Arc<Producer<Result<T, E>>>, usize)>, 
        timeout: u32) -> FutTask<T, E> where T: Send + 'static, E: Send + 'static {
            let uid = self.counter.fetch_add(1, Ordering::SeqCst);
            let (p, c) = npnc_channel(1);
            let copy = self.executor;
            let func = Box::new(move || {
                callback(copy, Arc::new(p), uid);
            });
            (self.executor)(TaskType::Sync, 10000000, func, Atom::from(uid.to_string() + " future task"));
            FutTask::new(uid, timeout, Arc::new(c))
    }
}
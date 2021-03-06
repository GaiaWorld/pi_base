use std::boxed::FnBox;
use std::mem::transmute;
use std::collections::VecDeque;
use std::fmt::{Display, Formatter, Result};

use pi_lib::atom::Atom;

/*
* 任务类型
*/
#[derive(Copy, Clone, Debug)]
pub enum TaskType {
    Empty,      //空任务
    Async,      //异步任务
    Sync,       //同步任务
    SyncImme,   //同步立即任务
}

unsafe impl Send for TaskType {}

/*
* 任务结构
*/
pub struct Task {
    priority:       u64,                //任务优先级
    func:           (usize, usize),     //任务函数
    info:           Atom,               //任务信息
}

unsafe impl Sync for Task {} //声明保证多线程安全性

impl Display for Task {
	fn fmt(&self, f: &mut Formatter) -> Result {
		write!(f, "Task[priority = {}, func = {:?}, info = {}]", self.priority, self.func, *self.info)
	}
}

impl Task {
    pub fn new() -> Self {
        Task {
            priority:   0,
            func:       (0, 0),
            info:       Atom::from(""),
        }
    }

    pub fn copy_to(&self, dest: &mut Self) {
        //复制其它成员
        dest.priority = self.priority;
        dest.func = self.func;
        dest.info = self.info.clone();
    }
    
    pub fn get_priority(&self) -> u64 {
        self.priority
    }
    
    pub fn set_priority(&mut self, priority: u64) {
        self.priority = priority;
    }
    
    pub fn set_func(&mut self, func: Option<Box<FnBox()>>) {
        match func {
            Some(f) => {
                let (x, y): (usize, usize) = unsafe { transmute(f) };
                self.func.0 = x;
                self.func.1 = y;
            },
            None => (),
        }
    }

    pub fn get_info(&self) -> &str {
        self.info.as_str()
    }

    pub fn set_info(&mut self, info: Atom) {
        self.info = info;
    }

    pub fn reset(&mut self) {
        self.priority = 0;
        self.func = (0, 0);
        self.info = Atom::from("");
    }

    pub fn run(&self) {
        if self.func == (0, 0) {
            return;
        }
        let func: Box<FnBox()> = unsafe { transmute(self.func) };
        func();
    }
}

/*
* 任务缓存结构
*/
pub struct TaskCache {
    cache: VecDeque<Task>, //任务缓存
}

impl Display for TaskCache {
	fn fmt(&self, f: &mut Formatter) -> Result {
		write!(f, "TaskCache[size = {}]", self.cache.len())
	}
}

impl TaskCache {
    pub fn new(len: u32) -> Self {
        if len < 1 {
            panic!("invalid task cache size");
        }

        let mut cache = VecDeque::with_capacity(len as usize);
        for _ in 0..len {
            cache.push_back(Task::new());
        }
        TaskCache {
            cache: cache,
        }
    }
    
    pub fn pop(&mut self) -> Task {
        match self.cache.pop_front() {
            Some(e) => e,
            None => Task::new(),
        }
    }
    
    pub fn push(&mut self, mut entry: Task) {
        entry.reset();
        self.cache.push_back(entry);
    }
    
    pub fn clean(&mut self) -> usize {
        //TODO...
        self.size()
    }
    
    pub fn size(&self) -> usize {
        self.cache.len()
    }
}


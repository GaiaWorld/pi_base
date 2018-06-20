use std::sync::Arc;
use std::sync::Mutex;
use std::thread;
use std::time::{Duration};
use std::sync::atomic::{AtomicIsize, AtomicUsize, Ordering, AtomicU64};
use std::boxed::FnBox;
use std::mem::transmute;

use pi_lib::wheel::{Wheel, Item};
use pi_lib::time::{now_millis};

lazy_static! {
	pub static ref TIMER: Timer = Timer::new(10);
}

#[derive(Clone)]
pub struct Timer{
	wheel: Arc<Mutex<Wheel<(usize, usize)>>>,
	statistics: Statistics,
	clock_ms: u64
}

impl Timer{
	pub fn new(clock_ms: u64) -> Self{
		let wheel = Arc::new(Mutex::new(Wheel::new()));
		Timer{
			wheel: wheel, 
			statistics: Statistics::new(),
			clock_ms: clock_ms,
		}
	}

	pub fn run(&self){
		let s = self.clone();
		thread::spawn(move ||{
			let wheel = s.wheel.clone();
			let mut sleep_time = s.clock_ms;
			wheel.lock().unwrap().set_time(now_millis());
			loop {
				thread::sleep(Duration::from_millis(sleep_time));
				let mut now = now_millis();
                loop {
                    let r = {
                        let mut w = wheel.lock().unwrap();
                        match now >= s.clock_ms + w.time{
                            true => w.roll(),
                            false => {
                                sleep_time = s.clock_ms + w.time- now;
                                break;
                            }
                        }
                    };
                    s.statistics.run_count.fetch_add(r.len(), Ordering::Relaxed);//统计运行任务个数
                    for v in r.into_iter(){
                        let func: Box<FnBox()> = unsafe { transmute(v.0.elem) };
                        func();
                    }
                    let old = now;
                    now = now_millis();
                    s.statistics.run_time.fetch_add(now - old, Ordering::Relaxed); //统计运行时长
                }
			}
		});
	}

	pub fn set_timeout(&self, f: Box<FnBox()>, ms: u64) -> Arc<AtomicIsize>{
		self.statistics.all_count.fetch_add(1, Ordering::Relaxed);
		TIMER.wheel.lock().unwrap().insert(Item{elem: unsafe { transmute(f) }, time_point: now_millis() + ms})
	}

	pub fn cancel(&self, index: Arc<AtomicIsize>) -> Option<Box<FnBox()>>{
		match self.wheel.lock().unwrap().try_remove(index) {
			Some(v) => {
                self.statistics.cancel_count.fetch_add(1, Ordering::Relaxed);
                unsafe { transmute(v.elem) }
            },
			None => {None},
		}
	}
}

#[derive(Clone)]
struct Statistics {
	pub all_count: Arc<AtomicUsize>,
    pub cancel_count: Arc<AtomicUsize>,
	pub run_count: Arc<AtomicUsize>,
	pub run_time: Arc<AtomicU64>,
}

impl Statistics{
	pub fn new() -> Statistics{
		Statistics{
			all_count: Arc::new(AtomicUsize::new(0)),
            cancel_count: Arc::new(AtomicUsize::new(0)),
			run_count: Arc::new(AtomicUsize::new(0)),
			run_time: Arc::new(AtomicU64::new(0)),
		}
	}
}

#[test]
fn test(){
	let f = ||{
		println!("test time:{}", "success");
	};
	TIMER.run();
	TIMER.set_timeout(Box::new(f), 1000);
	thread::sleep(Duration::from_millis(2000));
}


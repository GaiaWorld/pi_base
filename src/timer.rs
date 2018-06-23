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
	pub fn new(mut clock_ms: u64) -> Self{
		let wheel = Arc::new(Mutex::new(Wheel::new()));
        if clock_ms < 10{
            clock_ms = 10;
        }
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
                now = s.run_zero(now);//运行0毫秒任务
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
                    now = s.run_task(&r, now);
                    now = s.run_zero(now);//运行0毫秒任务
                }

			}
		});
	}

	pub fn set_timeout(&self, f: Box<FnBox()>, ms: u64) -> Arc<AtomicIsize>{
		self.statistics.all_count.fetch_add(1, Ordering::Relaxed);
        let mut w = TIMER.wheel.lock().unwrap();
        let time = w.time;
		w.insert(Item{elem: unsafe { transmute(f) }, time_point: time + ms})
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

    //执行任务，返回任务执行完的时间
    fn run_task(&self, r: &Vec<(Item<(usize, usize)>, Arc<AtomicIsize>)>, old: u64) -> u64{
        self.statistics.run_count.fetch_add(r.len(), Ordering::Relaxed);//统计运行任务个数
        for v in r.iter(){
            let func: Box<FnBox()> = unsafe { transmute(v.0.elem) };
            func();
        }
        let now = now_millis();
        self.statistics.run_time.fetch_add(now - old, Ordering::Relaxed); //统计运行时长
        now
    }

    fn run_zero(&self, mut now: u64) -> u64{
        loop {
            let mut r = {
                let mut w = self.wheel.lock().unwrap();
                match w.zero_size() > 0{
                    true => w.get_zero(),
                    false => {
                        break;
                    }
                }
            };
            now = self.run_task(&r, now);
            r.clear();
            self.wheel.lock().unwrap().set_zero_cache(r);
        }
        now
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
    TIMER.run();
    //thread::sleep(Duration::from_millis(8));
    let now = now_millis();
	let f = move||{
        //let n = now_millis();
		// println!("test time:{}", n - now);
        // println!("run_time-------------{}", TIMER.statistics.run_time.load(Ordering::Relaxed));
	};
    TIMER.set_timeout(Box::new(f), 10);
	//let index = TIMER.set_timeout(Box::new(f), 1000);
    //println!("index-------------{}", index.load(Ordering::Relaxed));
	thread::sleep(Duration::from_millis(500));
}


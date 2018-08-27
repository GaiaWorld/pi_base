use std::panic::resume_unwind;
use std::sync::Arc;

use futures::*;
use npnc::ConsumeError;
use npnc::bounded::mpmc::Consumer;

use util::now_millisecond;

/*
* 未来任务
*/
#[derive(Debug)]
pub struct FutTask<T, E> {
    uid:        usize,                          //未来任务id
    timeout:    i64,                            //未来任务超时时间
    inner:      Arc<Consumer<Result<T, E>>>,    //内部未来任务
}

impl<T: Send + 'static, E: Send + 'static> FutTask<T, E> {
    //构建一个未来任务
    pub fn new(uid: usize, timeout: u32, inner: Arc<Consumer<Result<T, E>>>) -> Self {
        FutTask {
            uid: uid,
            inner: inner,
            timeout: now_millisecond() + timeout as i64,
        }
    }

    //获取当前未来任务id
    pub fn get_uid(&self) -> usize {
        self.uid
    }
}

impl<T: Send + 'static, E: Send + 'static> Future for FutTask<T, E> {
    type Item = T;
    type Error = E;

    fn poll(&mut self) -> Poll<T, E> {
        if self.timeout < now_millisecond() {
            resume_unwind(Box::new("future task timeout")) //超时
        } else {
            match self.inner.consume() {
                Ok(Ok(r)) => Ok(Async::Ready(r)),
                Ok(Err(e)) => Err(e),
                Err(e) => {
                    match e {
                        ConsumeError::Empty => Ok(Async::NotReady), //还未准备好
                        _ => resume_unwind(Box::new("future task failed")), //异常
                    }
                },
            }
        }
    }
}
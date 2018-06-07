use std::boxed::FnBox;
use std::time::Duration;
use std::path::Path;
use std::sync::{Arc, Mutex, Condvar};
use std::fmt::{Debug, Formatter, Result as FmtResult};
use std::fs::{File, OpenOptions, Metadata, rename, remove_file};
use std::io::{Seek, Read, Write, Result, SeekFrom, Error, ErrorKind};

use pi_vm::task::TaskType;
use pi_vm::task_pool::TaskPool;
use pi_lib::atom::Atom;

/*
* 文件块默认大小
*/
const BLOCK_SIZE: usize = 8192;

/*
* 文件异步访问任务类型
*/
const ASYNC_FILE_TASK_TYPE: TaskType = TaskType::Sync;

/*
* 文件异步访问任务优先级
*/
const OPEN_ASYNC_FILE_PRIORITY: u64 = 10;

/*
* 文件异步访问任务优先级
*/
const READ_ASYNC_FILE_PRIORITY: u64 = 100;

/*
* 文件异步访问任务优先级
*/
const WRITE_ASYNC_FILE_PRIORITY: u64 = 60;

/*
* 重命名文件优先级
*/
const RENAME_ASYNC_FILE_PRIORITY: u64 = 30;

/*
* 移除文件任务优先级
*/
const REMOVE_ASYNC_FILE_PRIORITY: u64 = 10;

/*
* 打开异步文件信息
*/
const OPEN_ASYNC_FILE_INFO: &str = "open asyn file";

/*
* 读异步文件信息
*/
const READ_ASYNC_FILE_INFO: &str = "read asyn file";

/*
* 写异步文件信息
*/
const WRITE_ASYNC_FILE_INFO: &str = "write asyn file";

/*
* 重命名文件
*/
const RENAME_ASYNC_FILE_INFO: &str = "rename asyn file";

/*
* 移除文件信息
*/
const REMOVE_ASYNC_FILE_INFO: &str = "remove asyn file";

/*
* 存储任务池
*/
lazy_static! {
	pub static ref STORE_TASK_POOL: Arc<(Mutex<TaskPool>, Condvar)> = Arc::new((Mutex::new(TaskPool::new(10)), Condvar::new()));
}

/*
* 文件选项
*/
pub enum AsynFileOptions {
    OnlyRead(u8),
    OnlyWrite(u8),
    OnlyAppend(u8),
    ReadAppend(u8),
    ReadWrite(u8),
}

/*
* 写文件选项
*/
pub enum WriteOptions {
    None,
    Flush,
    Sync(bool),
    SyncAll(bool),
}

/*
* 异步文件
*/
pub struct AsyncFile{
    inner: File, 
    buffer_size: usize, 
    pos: u64, 
    buffer: Option<Vec<u8>>,
}

impl Debug for AsyncFile {
    fn fmt(&self, f: &mut Formatter) -> FmtResult {
        write!(f, "AsyncFile[file = {:?}, buffer_size = {}, current_pos = {}, buffer_len = {}, buffer_size = {}]", 
            self.inner, self.buffer_size, self.pos, self.buffer.as_ref().unwrap().len(), self.buffer.as_ref().unwrap().capacity())
    }
}

impl AsyncFile {
    //以指定方式打开指定文件
    pub fn open<P: AsRef<Path> + Send + 'static>(path: P, options: AsynFileOptions, callback: Box<FnBox(Result<Self>)>) {
        let func = move || {
            let (r, w, a, c, len) = match options {
                AsynFileOptions::OnlyRead(len) => (true, false, false, false, len),
                AsynFileOptions::OnlyWrite(len) => (false, true, false, true, len),
                AsynFileOptions::OnlyAppend(len) => (false, false, true, true, len),
                AsynFileOptions::ReadAppend(len) => (true, false, true, true, len),
                AsynFileOptions::ReadWrite(len) => (true, true, false, true, len),
            };

            match OpenOptions::new()
                            .read(r)
                            .write(w)
                            .append(a)
                            .create(c)
                            .open(path) {
                Err(e) => callback(Err(e)),
                Ok(file) => {
                    let buffer_size = match file.metadata() {
                        Ok(meta) => get_block_size(&meta) * len as usize,
                        _ => BLOCK_SIZE * len as usize,
                    };
                    callback(Ok(AsyncFile {
                                            inner: file, 
                                            buffer_size: buffer_size, 
                                            pos: 0, 
                                            buffer: Some(Vec::with_capacity(0))
                                        }))
                },
            }
        };

        let &(ref lock, ref cvar) = &**STORE_TASK_POOL;
        let mut task_pool = lock.lock().unwrap();
        (*task_pool).push(ASYNC_FILE_TASK_TYPE, OPEN_ASYNC_FILE_PRIORITY, Box::new(func), Atom::from(OPEN_ASYNC_FILE_INFO));
        cvar.notify_one();
    }

    //文件重命名
    pub fn rename<P: AsRef<Path> + Clone + Send + 'static>(from: P, to: P, callback: Box<FnBox(P, P, Result<()>)>) {
        let func = move || {
            let result = rename(from.clone(), to.clone());
            callback(from, to, result);
        };

        let &(ref lock, ref cvar) = &**STORE_TASK_POOL;
        let mut task_pool = lock.lock().unwrap();
        (*task_pool).push(ASYNC_FILE_TASK_TYPE, RENAME_ASYNC_FILE_PRIORITY, Box::new(func), Atom::from(RENAME_ASYNC_FILE_INFO));
        cvar.notify_one();
    }

    //移除指定文件
    pub fn remove<P: AsRef<Path> + Send + 'static>(path: P, callback: Box<FnBox(Result<()>)>) {
        let func = move || {
            let result = remove_file(path);
            callback(result);
        };

        let &(ref lock, ref cvar) = &**STORE_TASK_POOL;
        let mut task_pool = lock.lock().unwrap();
        (*task_pool).push(ASYNC_FILE_TASK_TYPE, REMOVE_ASYNC_FILE_PRIORITY, Box::new(func), Atom::from(REMOVE_ASYNC_FILE_INFO));
        cvar.notify_one();
    }

    //检查是否是符号链接
    pub fn is_symlink(&self) -> bool {
        self.inner.metadata().ok().unwrap().file_type().is_symlink()
    }

    //检查是否是文件
    pub fn is_file(&self) -> bool {
        self.inner.metadata().ok().unwrap().file_type().is_file()
    }

    //检查文件是否只读
    pub fn is_only_read(&self) -> bool {
        self.inner.metadata().ok().unwrap().permissions().readonly()
    }
    
    //获取文件大小
    pub fn get_size(&self) -> u64 {
        self.inner.metadata().ok().unwrap().len()
    }

    //获取文件修改时间
    pub fn get_modified_time(&self) -> Option<Duration> {
        match self.inner.metadata().ok().unwrap().modified() {
            Ok(time) => {
                match time.elapsed() {
                    Ok(duration) => Some(duration),
                    _ => None,
                }
            },
            _ => None,
        }
    }

    //获取文件访问时间
    pub fn get_accessed_time(&self) -> Option<Duration> {
        match self.inner.metadata().ok().unwrap().accessed() {
            Ok(time) => {
                match time.elapsed() {
                    Ok(duration) => Some(duration),
                    _ => None,
                }
            },
            _ => None,
        }
    }

    //获取文件创建时间
    pub fn get_created_time(&self) -> Option<Duration> {
        match self.inner.metadata().ok().unwrap().created() {
            Ok(time) => {
                match time.elapsed() {
                    Ok(duration) => Some(duration),
                    _ => None,
                }
            },
            _ => None,
        }
    }

    //从指定位置开始，读指定字节
    pub fn read(mut self, pos: u64, len: usize, callback: Box<FnBox(Self, Result<Vec<u8>>)>) {
        let func = move || {
            let file_size = self.get_size();
            if file_size == 0 || len == 0 {
                let vec = self.buffer.take().unwrap();
                callback(init_read_file(self), Ok(vec));
                return;
            } else {
                self = alloc_buffer(self, file_size, len);
            }
            
            //保证在append时，当前位置也不会被改变
            match self.inner.seek(SeekFrom::Start(pos as u64)) {
                Err(e) => callback(init_read_file(self), Err(e)),
                Ok(_) => {
                    let buf_cap = self.buffer.as_ref().unwrap().capacity() as isize;
                    match  buf_cap - self.pos as isize {
                        diff if diff > 0 => {
                            let buf_size = if diff as usize >= self.buffer_size {
                                self.buffer_size
                            } else {
                                diff as usize
                            };
                            
                            match self.inner.read(&mut self.buffer.as_mut().unwrap()[(self.pos as usize)..(self.pos as usize + buf_size)]) {
                                Ok(n) if n == 0 || n < buf_size => {
                                    //文件尾
                                    self.pos = self.buffer.as_ref().unwrap().len() as u64;
                                    let vec = self.buffer.take().unwrap();
                                    callback(init_read_file(self), Ok(vec));
                                },
                                Ok(n) => {
                                    self.pos += n as u64;
                                    if self.pos >= buf_cap as u64 {
                                        //读完成
                                        let vec = self.buffer.take().unwrap();
                                        callback(init_read_file(self), Ok(vec));
                                    } else {
                                        //继续读
                                        self.read(pos + n as u64, len - n, callback);
                                    }
                                },
                                Err(ref e) if e.kind() == ErrorKind::Interrupted => {
                                    //重复读
                                    self.read(pos, len, callback);
                                },
                                Err(e) => callback(init_read_file(self), Err(e)),
                            }
                        },
                        _ => {
                            //读完成
                            let vec = self.buffer.take().unwrap();
                            callback(init_read_file(self), Ok(vec));
                        },
                    }       
                },
            }
        };

        let &(ref lock, ref cvar) = &**STORE_TASK_POOL;
        let mut task_pool = lock.lock().unwrap();
        (*task_pool).push(ASYNC_FILE_TASK_TYPE, READ_ASYNC_FILE_PRIORITY, Box::new(func), Atom::from(READ_ASYNC_FILE_INFO));
        cvar.notify_one();
    }

    //从指定位置开始，写指定字节
    pub fn write(mut self, options: WriteOptions, pos: u64, bytes: Vec<u8>, callback: Box<FnBox(Self, Result<()>)>) {
        let func = move || {
            if !&bytes[self.pos as usize..].is_empty() {
                match self.inner.seek(SeekFrom::Start(pos as u64)) {
                    Err(e) => callback(init_write_file(self), Err(e)),
                    Ok(_) => {
                        match self.inner.write(&bytes[self.pos as usize..]) {
                            Ok(0) => {
                                callback(init_write_file(self), Err(Error::new(ErrorKind::WriteZero, "write failed")));
                            },
                            Ok(n) => {
                                //继续写
                                self.pos += n as u64;
                                self.write(options, pos + n as u64, bytes, callback);
                            },
                            Err(ref e) if e.kind() == ErrorKind::Interrupted => {
                                //重复写
                                self.write(options, pos, bytes, callback);
                            },
                            Err(e) => {
                                callback(init_write_file(self), Err(e));
                            },
                        }
                    },
                }
            } else {
                //写完成
                let result = match options {
                    WriteOptions::None => Ok(()),
                    WriteOptions::Flush => self.inner.flush(),
                    WriteOptions::Sync(true) => self.inner.flush().and_then(|_| self.inner.sync_data()),
                    WriteOptions::Sync(false) => self.inner.sync_data(),
                    WriteOptions::SyncAll(true) => self.inner.flush().and_then(|_| self.inner.sync_all()),
                    WriteOptions::SyncAll(false) => self.inner.sync_all(),
                };
                callback(init_write_file(self), result);
            }
        };

        let &(ref lock, ref cvar) = &**STORE_TASK_POOL;
        let mut task_pool = lock.lock().unwrap();
        (*task_pool).push(ASYNC_FILE_TASK_TYPE, WRITE_ASYNC_FILE_PRIORITY, Box::new(func), Atom::from(WRITE_ASYNC_FILE_INFO));
        cvar.notify_one();
    }

    //复制异步文件
    pub unsafe fn try_clone(&self) -> Result<Self> {
        match self.inner.try_clone() {
            Err(e) => Err(e),
            Ok(inner) => {
                Ok(AsyncFile {
                    inner: inner, 
                    buffer_size: self.buffer_size, 
                    pos: 0, 
                    buffer: Some(Vec::with_capacity(0))
                })
            },
        }
    }
}

#[inline]
fn init_read_file(mut file: AsyncFile) -> AsyncFile {
    file.pos = 0;
    file.buffer = Some(Vec::with_capacity(0));
    file
}

#[inline]
fn init_write_file(mut file: AsyncFile) -> AsyncFile {
    file.pos = 0;
    file
}

#[inline]
fn alloc_buffer(mut file: AsyncFile, file_size: u64, len: usize) -> AsyncFile {
    if file.buffer.as_ref().unwrap().len() == 0 {
        if file_size > len as u64 {
            file.buffer.as_mut().unwrap().reserve(len);
            file.buffer.as_mut().unwrap().resize(len, 0);
        } else {
            file.buffer.as_mut().unwrap().reserve(file_size as usize);
            file.buffer.as_mut().unwrap().resize(file_size as usize, 0);
        }
    }
    file
}

#[cfg(unix)]
fn get_block_size(meta: &Metadata) -> usize {
    use std::os::unix::fs::MetadataExt;
    metadata.blksize() as usize
}

#[cfg(not(unix))]
fn get_block_size(_meta: &Metadata) -> usize {
    BLOCK_SIZE
}
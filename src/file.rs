use std::sync::Arc;
use std::path::Path;
use std::clone::Clone;
use std::boxed::FnBox;
use std::time::Duration;
#[cfg(any(unix))]
use std::os::unix::fs::FileExt;
#[cfg(any(windows))]
use std::os::windows::fs::FileExt;

use std::fmt::{Debug, Formatter, Result as FmtResult};
use std::fs::{File, OpenOptions, Metadata, rename, remove_file};
use std::io::{Seek, Read, Write, Result, SeekFrom, Error, ErrorKind};

use pi_lib::atom::Atom;

use task::TaskType;
use pi_base_impl::cast_store_task;

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
* 共享读异步文件信息
*/
const SHARED_READ_ASYNC_FILE_INFO: &str = "shared read asyn file";

/*
* 写异步文件信息
*/
const WRITE_ASYNC_FILE_INFO: &str = "write asyn file";

/*
* 共享写异步文件信息
*/
const SHARED_WRITE_ASYNC_FILE_INFO: &str = "shared write asyn file";

/*
* 重命名文件
*/
const RENAME_ASYNC_FILE_INFO: &str = "rename asyn file";

/*
* 移除文件信息
*/
const REMOVE_ASYNC_FILE_INFO: &str = "remove asyn file";

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
* 共享接口
*/
pub trait Shared {
    type T;

    //通过异步文件构建共享异步文件
    fn new(file: Self::T) -> Arc<Self::T>;

    //原子的从指定位置开始读指定字节
    fn pread(self, pos: u64, len: usize, callback: Box<FnBox(Arc<Self::T>, Result<Vec<u8>>)>);

    //原子的从指定位置开始写指定字节
    fn pwrite(mut self, options: WriteOptions, pos: u64, bytes: Vec<u8>, callback: Box<FnBox(Arc<Self::T>, Result<usize>)>);
}

/*
* 共享异步文件
*/
pub type SharedFile = Arc<AsyncFile>;

impl Shared for SharedFile {
    type T = AsyncFile;

    fn new(file: Self::T) -> Arc<Self::T> {
        Arc::new(file)
    }

    fn pread(self, pos: u64, len: usize, callback: Box<FnBox(Arc<Self::T>, Result<Vec<u8>>)>) {
        if len == 0 {
            return callback(self, Err(Error::new(ErrorKind::Other, "pread failed, invalid len")));
        }

        let func = move || {
            let mut vec: Vec<u8> = Vec::with_capacity(len);
            vec.resize(len, 0);

            #[cfg(any(unix))]
            let r = self.inner.read_at(&mut vec[..], pos);
            #[cfg(any(windows))]
            let r = self.inner.seek_read(&mut vec[..], pos);

            match r {
                Ok(short_len) if short_len < len => {
                    //继续读
                    pread_continue(vec, self, pos + short_len as u64, len - short_len, callback);
                },
                Ok(_len) => {
                    //读完成
                    callback(self, Ok(vec))
                },
                Err(ref e) if e.kind() == ErrorKind::Interrupted => {
                    //重复读
                    self.pread(pos, len, callback);
                },
                Err(e) => callback(self, Err(e)),
            }
        };
        cast_store_task(ASYNC_FILE_TASK_TYPE, READ_ASYNC_FILE_PRIORITY, Box::new(func), Atom::from(SHARED_READ_ASYNC_FILE_INFO));
    }

    fn pwrite(mut self, options: WriteOptions, pos: u64, bytes: Vec<u8>, callback: Box<FnBox(Arc<Self::T>, Result<usize>)>) {
        let len = bytes.len();
        if len == 0 {
            return callback(self, Ok(0));
        }

        let func = move || {
            #[cfg(any(unix))]
            let r = self.inner.write_at(&bytes[..], pos);
            #[cfg(any(windows))]
            let r = self.inner.seek_write(&bytes[..], pos);

            match r {
                Ok(short_len) if short_len < len => {
                    //继续写
                    pwrite_continue(len - short_len, self, options, pos + short_len as u64, bytes, callback);
                },
                Ok(len) => {
                    //写完成
                    let result = match options {
                        WriteOptions::None => Ok(len),
                        WriteOptions::Flush => Arc::make_mut(&mut self).inner.flush().and(Ok(len)),
                        WriteOptions::Sync(true) => Arc::make_mut(&mut self).inner.flush().and_then(|_| self.inner.sync_data()).and(Ok(len)),
                        WriteOptions::Sync(false) => Arc::make_mut(&mut self).inner.sync_data().and(Ok(len)),
                        WriteOptions::SyncAll(true) => Arc::make_mut(&mut self).inner.flush().and_then(|_| self.inner.sync_all()).and(Ok(len)),
                        WriteOptions::SyncAll(false) => Arc::make_mut(&mut self).inner.sync_all().and(Ok(len)),
                    };
                    callback(self, result);
                },
                Err(ref e) if e.kind() == ErrorKind::Interrupted => {
                    //重复写
                    self.pwrite(options, pos, bytes, callback);
                },
                Err(e) => callback(self, Err(e)),
            }
        };
        cast_store_task(ASYNC_FILE_TASK_TYPE, WRITE_ASYNC_FILE_PRIORITY, Box::new(func), Atom::from(SHARED_WRITE_ASYNC_FILE_INFO));
    }
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

impl Clone for AsyncFile {
    fn clone(&self) -> Self {
        match self.inner.try_clone() {
            Err(e) => panic!("{:?}", e),
            Ok(inner) => {
                AsyncFile {
                    inner: inner, 
                    buffer_size: self.buffer_size, 
                    pos: 0, 
                    buffer: Some(Vec::with_capacity(0))
                }
            },
        }
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
        cast_store_task(ASYNC_FILE_TASK_TYPE, OPEN_ASYNC_FILE_PRIORITY, Box::new(func), Atom::from(OPEN_ASYNC_FILE_INFO));
    }

    //文件重命名
    pub fn rename<P: AsRef<Path> + Clone + Send + 'static>(from: P, to: P, callback: Box<FnBox(P, P, Result<()>)>) {
        let func = move || {
            let result = rename(from.clone(), to.clone());
            callback(from, to, result);
        };
        cast_store_task(ASYNC_FILE_TASK_TYPE, RENAME_ASYNC_FILE_PRIORITY, Box::new(func), Atom::from(RENAME_ASYNC_FILE_INFO));
    }

    //移除指定文件
    pub fn remove<P: AsRef<Path> + Send + 'static>(path: P, callback: Box<FnBox(Result<()>)>) {
        let func = move || {
            let result = remove_file(path);
            callback(result);
        };
        cast_store_task(ASYNC_FILE_TASK_TYPE, REMOVE_ASYNC_FILE_PRIORITY, Box::new(func), Atom::from(REMOVE_ASYNC_FILE_INFO));
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
        cast_store_task(ASYNC_FILE_TASK_TYPE, READ_ASYNC_FILE_PRIORITY, Box::new(func), Atom::from(READ_ASYNC_FILE_INFO));
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
        cast_store_task(ASYNC_FILE_TASK_TYPE, WRITE_ASYNC_FILE_PRIORITY, Box::new(func), Atom::from(WRITE_ASYNC_FILE_INFO));
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

#[cfg(any(unix))]
fn get_block_size(meta: &Metadata) -> usize {
    use std::os::unix::fs::MetadataExt;
    meta.blksize() as usize
}

#[cfg(any(windows))]
fn get_block_size(_meta: &Metadata) -> usize {
    BLOCK_SIZE
}

fn pread_continue(mut vec: Vec<u8>, file: SharedFile, pos: u64, len: usize, callback: Box<FnBox(Arc<<SharedFile as Shared>::T>, Result<Vec<u8>>)>) {
    let func = move || {
        #[cfg(any(unix))]
        let r = file.inner.read_at(&mut vec[pos as usize..(pos as usize + len)], pos);
        #[cfg(any(windows))]
        let r = file.inner.seek_read(&mut vec[pos as usize..(pos as usize + len)], pos);

        match r {
            Ok(short_len) if short_len < len => {
                //继续读
                pread_continue(vec, file, pos + short_len as u64, len - short_len, callback);
            },
            Ok(_len) => {
                //读完成
                callback(file, Ok(vec))
            },
            Err(ref e) if e.kind() == ErrorKind::Interrupted => {
                //重复读
                file.pread(pos, len, callback);
            },
            Err(e) => callback(file, Err(e)),
        }
    };
    cast_store_task(ASYNC_FILE_TASK_TYPE, READ_ASYNC_FILE_PRIORITY, Box::new(func), Atom::from(SHARED_READ_ASYNC_FILE_INFO));
}

fn pwrite_continue(len: usize, mut file: SharedFile, options: WriteOptions, pos: u64, bytes: Vec<u8>, callback: Box<FnBox(Arc<<SharedFile as Shared>::T>, Result<usize>)>) {
    let func = move || {
        #[cfg(any(unix))]
        let r = file.inner.write_at(&bytes[pos as usize..len], pos);
        #[cfg(any(windows))]
        let r = file.inner.seek_write(&bytes[pos as usize..len], pos);

        match r {
            Ok(short_len) if short_len < len => {
                //继续写
                pwrite_continue(len - short_len, file, options, pos + short_len as u64, bytes, callback);
            },
            Ok(len) => {
                //写完成
                let result = match options {
                    WriteOptions::None => Ok(len),
                    WriteOptions::Flush => Arc::make_mut(&mut file).inner.flush().and(Ok(len)),
                    WriteOptions::Sync(true) => Arc::make_mut(&mut file).inner.flush().and_then(|_| file.inner.sync_data()).and(Ok(len)),
                    WriteOptions::Sync(false) => Arc::make_mut(&mut file).inner.sync_data().and(Ok(len)),
                    WriteOptions::SyncAll(true) => Arc::make_mut(&mut file).inner.flush().and_then(|_| file.inner.sync_all()).and(Ok(len)),
                    WriteOptions::SyncAll(false) => Arc::make_mut(&mut file).inner.sync_all().and(Ok(len)),
                };
                callback(file, result);
            },
            Err(ref e) if e.kind() == ErrorKind::Interrupted => {
                //重复写
                file.pwrite(options, pos, bytes, callback);
            },
            Err(e) => callback(file, Err(e)),
        }
    };
    cast_store_task(ASYNC_FILE_TASK_TYPE, WRITE_ASYNC_FILE_PRIORITY, Box::new(func), Atom::from(SHARED_WRITE_ASYNC_FILE_INFO));
}
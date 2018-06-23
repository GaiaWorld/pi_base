use std::any::Any;
use std::sync::Arc;
use std::boxed::FnBox;

use pi_lib::atom::Atom;

/*
* 环境
*/
pub trait Env {
    //获取属性
    fn get_attr(&self, key: Atom) -> Option<Box<Any>>;

    //设置属性，返回上个属性值
    fn set_attr(&mut self, key: Atom, value: Box<Any>) -> Option<Box<Any>>;
}

/*
* ES5Type
*/
pub trait ES5Type {
    //获取指定类型的类型id
    fn get_type_id(&self, value: u32) -> u8;

    //获取内部值
    fn get_value(&self) -> usize;

    //判断是否是无效值
	fn is_none(&self) -> bool;

    //判断是否是undefined
	fn is_undefined(&self) -> bool;

    //判断是否是null
    fn is_null(&self) -> bool;

    //判断是否是boolean
	fn is_boolean(&self) -> bool;

    //判断是否是数字
	fn is_number(&self) -> bool;

    //判断是否是字符串
	fn is_string(&self) -> bool;

    //判断是否是对象
	fn is_object(&self) -> bool;

    //判断是否是数组
	fn is_array(&self) -> bool;

    //判断是否是ArrayBuffer
	fn is_array_buffer(&self) -> bool;

    //判断是否是Uint8Array
	fn is_uint8_array(&self) -> bool;

    //判断是否是NativeObject
	fn is_native_object(&self) -> bool;

    //获取boolean
    fn get_boolean(&self) -> bool;

    //获取i8
    fn get_i8(&self) -> i8;

    //获取i16
	fn get_i16(&self) -> i16;

    //获取i32
	fn get_i32(&self) -> i32;

    //获取i64
	fn get_i64(&self) -> i64;

    //获取u8
	fn get_u8(&self) -> u8;

    //获取u16
	fn get_u16(&self) -> u16;

    //获取u32
	fn get_u32(&self) -> u32;

    //获取u64
	fn get_u64(&self) -> u64;

    //获取f32
	fn get_f32(&self) -> f32;

    //获取f64
	fn get_f64(&self) -> f64;

    //获取字符串
	fn get_str(&self) -> String;

    //获取对象指定域的值
	fn get_field(&self, key: String) -> Self;

    //获取数组长度
    fn get_array_length(&self) -> usize;

    //获取数组指定偏移的值
	fn get_index(&self, index: u32) -> Self;

    //获取指定Buffer的引用
    fn to_bytes(&self) -> &[u8];

    //获取指定Buffer的引用
    unsafe fn to_bytes_mut(&mut self) -> &mut [u8];

    //获取指定Buffer的复制
	fn into_vec(&self) -> Vec<u8>;

    //重置指定的Buffer
	fn from_bytes(&self, bytes: &[u8]);

    //获取NativeObject
	fn get_native_object(&self) -> usize;
}

/*
* ES5
*/
pub trait ES5 {
    type Type: ES5Type;

    //构建undefined
    fn new_undefined(&self) -> Self::Type;

    //构建null
    fn new_null(&self) -> Self::Type;

    //构建boolean
    fn new_boolean(&self, b: bool) -> Self::Type;

    //构建i8
    fn new_i8(&self, num: i8) -> Self::Type;

    //构建i16
    fn new_i16(&self, num: i16) -> Self::Type;

    //构建i32
    fn new_i32(&self, num: i32) -> Self::Type;

    //构建i64
    fn new_i64(&self, num: i64) -> Self::Type;

    //构建u8
    fn new_u8(&self, num: u8) -> Self::Type;

    //构建u16
    fn new_u16(&self, num: u16) -> Self::Type;

    //构建u32
    fn new_u32(&self, num: u32) -> Self::Type;

    //构建u64
    fn new_u64(&self, num: u64) -> Self::Type;

    //构建f32
    fn new_f32(&self, num: f32) -> Self::Type;

    //构建f64
    fn new_f64(&self, num: f64) -> Self::Type;

    //构建字符串，注意rust的字符串默认是UTF8编码，而JS是UTF16编码
    fn new_str(&self, str: String) -> Self::Type;

    //构建对象
    fn new_object(&self) -> Self::Type;

    //设置指定对象的域
    fn set_field(&self, object: &Self::Type, key: String, value: &Self::Type) -> bool;

    //构建数组
    fn new_array(&self) -> Self::Type;

    //设置指定数组指定偏移的值
    fn set_index(&self, array: &Self::Type, index: u32, value: &Self::Type) -> bool;

    //构建ArrayBuffer
    fn new_array_buffer(&self, length: u32) -> Self::Type;

    //构建Uint8Array
    fn new_uint8_array(&self, length: u32) -> Self::Type;

    //构建NativeObject
    fn new_native_object(&self, instance: usize) -> Self::Type;
}

/*
* 通用处理器
*/
pub trait Handler {
    type VM: ES5;

    //处理方法
    fn handle(&self, env: Arc<Env>, topic: Atom, args: Box<FnBox(Arc<Self::VM>) -> usize>);
}

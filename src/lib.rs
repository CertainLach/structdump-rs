use std::fmt::Write;
use std::{collections::HashMap, path::PathBuf, rc::Rc};

pub trait Codegen {
	fn gen_code(&self, res: &mut CodegenResult, out: &mut String);
}
impl<'a, T> Codegen for &'a T
where
	T: Codegen,
{
	fn gen_code(&self, res: &mut CodegenResult, out: &mut String) {
		(*self).gen_code(res, out)
	}
}

#[derive(Default)]
pub struct CodegenResult {
	codes: HashMap<String, usize>,
	id: usize,
	snippets: Vec<String>,
}
impl CodegenResult {
	pub fn add_code(&mut self, code: String, ty: &str) -> String {
		let var_name = if !self.codes.contains_key(&code) {
			let value = self.id;
			self.id += 1;
			self.codes.insert(code.clone(), value);
			let var_name = format!("code_{:x}", value);
			self.snippets
				.push(format!("let {}{} = {};", var_name, ty, code));
			var_name
		} else {
			let value = self.codes.get(&code).unwrap();
			format!("code_{:x}", value)
		};
		format!("{}.clone()", var_name)
	}
	pub fn add_rc_value<T: Codegen + ?Sized>(&mut self, s: &T) -> String {
		let mut out = String::new();
		s.gen_code(self, &mut out);
		self.add_code(format!("::std::rc::Rc::new({})", out), "")
	}
	pub fn add_into_rc_value(&mut self, s: &impl Codegen, ty: &'static str) -> String {
		let mut out = String::new();
		s.gen_code(self, &mut out);
		self.add_code(
			format!("{}.into()", out),
			&format!(": ::std::rc::Rc<{}>", ty),
		)
	}
	pub fn add_value(&mut self, s: &impl Codegen) -> String {
		let mut out = String::new();
		s.gen_code(self, &mut out);
		out
	}
	pub fn codegen(&mut self, s: &impl Codegen) -> String {
		let mut out = String::new();
		s.gen_code(self, &mut out);
		let mut real_out = String::new();
		real_out.push_str("{");
		for snip in self.snippets.iter() {
			real_out.push_str(&snip);
			real_out.push_str("\n");
		}
		real_out.push_str(&out);
		real_out.push_str("}");
		real_out
	}
}

impl<T: Codegen> Codegen for &Option<T> {
	fn gen_code(&self, res: &mut CodegenResult, out: &mut String) {
		write!(out, "std::option::Option::").unwrap();
		if self.is_some() {
			let v = self.as_ref().unwrap();
			write!(out, "Some({})", res.add_value(v)).unwrap();
		} else {
			write!(out, "None").unwrap();
		}
	}
}
impl Codegen for Rc<str> {
	fn gen_code(&self, res: &mut CodegenResult, out: &mut String) {
		let s: &str = &*self;
		write!(out, "{}", res.add_into_rc_value(&s, "str")).unwrap()
	}
}
impl<T: Codegen> Codegen for Rc<T> {
	fn gen_code(&self, res: &mut CodegenResult, out: &mut String) {
		let v: &T = &**self;
		out.push_str(&res.add_rc_value(v));
	}
}
impl<T: Codegen> Codegen for Vec<T> {
	fn gen_code(&self, res: &mut CodegenResult, out: &mut String) {
		out.push_str("vec![");
		for i in self.iter() {
			out.push_str(&res.add_value(i));
			out.push(',');
		}
		out.push_str("]");
	}
}
impl Codegen for &str {
	fn gen_code(&self, _res: &mut CodegenResult, out: &mut String) {
		out.push('"');
		out.push_str(&self.escape_debug().collect::<String>());
		out.push('"');
	}
}
impl Codegen for String {
	fn gen_code(&self, res: &mut CodegenResult, out: &mut String) {
		(&self as &str).gen_code(res, out)
	}
}
impl Codegen for PathBuf {
	fn gen_code(&self, _res: &mut CodegenResult, out: &mut String) {
		out.push_str("std::path::PathBuf::from(\"");
		out.push_str(&self.to_str().unwrap().escape_debug().collect::<String>());
		out.push_str("\")");
	}
}
impl Codegen for &bool {
	fn gen_code(&self, _res: &mut CodegenResult, out: &mut String) {
		write!(out, "{}", self).unwrap()
	}
}

macro_rules! num_impl {
	($t:ty) => {
		impl Codegen for &$t {
			fn gen_code(&self, _res: &mut CodegenResult, out: &mut String) {
				write!(out, "{}{}", self, stringify!($t)).unwrap();
			}
		}
	};
}
num_impl!(u8);
num_impl!(u16);
num_impl!(u32);
num_impl!(u64);
num_impl!(i8);
num_impl!(i16);
num_impl!(i32);
num_impl!(i64);
num_impl!(f32);
num_impl!(f64);
num_impl!(isize);
num_impl!(usize);

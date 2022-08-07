use std::{borrow::Cow, collections::HashMap, marker::PhantomData, path::PathBuf, rc::Rc};

use proc_macro2::Ident;

#[cfg(feature = "derive")]
pub use structdump_derive::Codegen;

// Hidden because only used by proc macros
#[doc(hidden)]
pub use proc_macro2::TokenStream;
#[doc(hidden)]
pub use quote::{format_ident, quote};

pub trait Codegen {
	fn gen_code(&self, res: &mut CodegenResult, unique: bool) -> TokenStream;
}
impl<'a, T> Codegen for &'a T
where
	T: Codegen,
{
	fn gen_code(&self, res: &mut CodegenResult, unique: bool) -> TokenStream {
		(*self).gen_code(res, unique)
	}
}

#[derive(Default)]
pub struct CodegenResult {
	snippets: Vec<TokenStream>,
	codes: HashMap<String, usize>,
	id: usize,
}
impl CodegenResult {
	pub fn add_code(
		&mut self,
		code: TokenStream,
		ty: Option<TokenStream>,
		unique: bool,
	) -> TokenStream {
		if unique {
			return code;
		}
		let code_str = code.to_string();
		#[allow(clippy::map_entry)]
		let var_name = if !self.codes.contains_key(&code_str) {
			let value = self.id;
			self.id += 1;
			self.codes.insert(code_str, value);
			let var_name = format_ident!("code_{:x}", value);
			let ty = ty.map(|t| quote! {: #t});
			self.snippets.push(quote! {
				let #var_name #ty = #code;
			});
			var_name
		} else {
			let value = self.codes.get(&code_str).unwrap();
			format_ident!("code_{:x}", value)
		};
		quote! {#var_name.clone()}
	}
	pub fn add_value(&mut self, s: impl Codegen, unique: bool) -> TokenStream {
		s.gen_code(self, unique)
	}
	pub fn codegen(&mut self, s: &impl Codegen, unique: bool) -> TokenStream {
		let fin_val = s.gen_code(self, unique);
		let snippets = self.snippets.iter();
		quote! {{
			#(#snippets)*
			#fin_val
		}}
	}
}

impl<T: Codegen> Codegen for Option<T> {
	fn gen_code(&self, res: &mut CodegenResult, unique: bool) -> TokenStream {
		if let Some(val) = self {
			let val = res.add_value(val, unique);
			quote! {
				structdump_import::Option::Some(#val)
			}
		} else {
			quote!(structdump_import::Option::None)
		}
	}
}
impl Codegen for Rc<str> {
	fn gen_code(&self, res: &mut CodegenResult, _unique: bool) -> TokenStream {
		let s: &str = self;
		res.add_code(
			quote! {
				<structdump_import::Rc<str>>::from(#s)
			},
			Some(quote![structdump_import::Rc<str>]),
			false,
		)
	}
}
impl<T: Codegen> Codegen for Rc<T> {
	fn gen_code(&self, res: &mut CodegenResult, _unique: bool) -> TokenStream {
		let v: &T = self;
		let v = res.add_value(v, true);
		res.add_code(
			quote! {
				structdump_import::Rc::new(#v)
			},
			Some(quote![structdump_import::Rc<_>]),
			false,
		)
	}
}
impl Codegen for Cow<'_, str> {
	fn gen_code(&self, res: &mut CodegenResult, _unique: bool) -> TokenStream {
		let v: &str = self;
		let v = res.add_value(v, true);
		quote! {structdump_import::Cow::Borrowed(#v)}
	}
}
impl<T: Codegen> Codegen for Vec<T> {
	fn gen_code(&self, res: &mut CodegenResult, unique: bool) -> TokenStream {
		let value = self
			.iter()
			.map(|v| res.add_value(v, unique))
			.collect::<Vec<_>>();
		res.add_code(
			quote! {
				structdump_import::vec![
					#(#value),*
				]
			},
			Some(quote![structdump_import::Vec<_>]),
			unique,
		)
	}
}
impl Codegen for &str {
	fn gen_code(&self, res: &mut CodegenResult, _unique: bool) -> TokenStream {
		let v: &str = self;
		// Strings are deduplicated, but lets keep output code smaller
		res.add_code(quote! {#v}, Some(quote![&'static str]), true)
	}
}
impl Codegen for bool {
	fn gen_code(&self, _res: &mut CodegenResult, _unique: bool) -> TokenStream {
		quote! {#self}
	}
}

impl Codegen for String {
	fn gen_code(&self, res: &mut CodegenResult, unique: bool) -> TokenStream {
		let v = res.add_value(self as &str, true);
		res.add_code(
			quote! {#v.to_owned()},
			Some(quote! {structdump_import::String}),
			unique,
		)
	}
}
impl Codegen for PathBuf {
	fn gen_code(&self, _res: &mut CodegenResult, _unique: bool) -> TokenStream {
		quote! {
			panic!("pathbuf is not supported")
		}
	}
}

macro_rules! num_impl {
    ($($t:ty)+) => {$(
        impl Codegen for $t {
            fn gen_code(&self, _res: &mut CodegenResult, _unique: bool) -> TokenStream {
				quote!{#self}
            }
        }
    )+};
}
num_impl!(u8 u16 u32 u64 u128 i8 i16 i32 i64 i128 isize usize f32 f64);

macro_rules! impl_tuple {
	($($gen:ident)*) => {
		#[allow(non_snake_case)]
		impl<$($gen,)*> Codegen for ($($gen,)*)
		where
			$($gen: Codegen,)*
		{
			fn gen_code(&self, res: &mut CodegenResult, unique: bool) -> TokenStream {
				let ($($gen,)*) = &self;
				let values: Vec<TokenStream> = vec![
					$({
						res.add_value($gen, unique)
					},)*
				];
				res.add_code(
					quote! {
						(#(#values,)*)
					},
					None,
					unique,
				)
			}
		}
	};
	($($cur:ident)* @ $c:ident $($rest:ident)*) => {
		impl_tuple!($($cur)*);
		impl_tuple!($($cur)* $c @ $($rest)*);
	};
	($($cur:ident)* @) => {
		impl_tuple!($($cur)*);
	}
}
impl_tuple! {
	@ A B C D E F G H I J K L
}

mod sealed {
	pub(super) trait StructType {}
	impl StructType for super::Named {}
	impl StructType for super::Unnamed {}
	impl StructType for super::Unit {}
}

pub struct Named;
pub struct Unnamed;
pub struct Unit;
pub struct StructBuilder<Type> {
	name: Ident,
	variant: Option<TokenStream>,
	fields: Vec<TokenStream>,
	unique: bool,
	_marker: PhantomData<Type>,
}
impl<Type> StructBuilder<Type> {
	pub fn new(name: Ident, variant: Option<Ident>, unique: bool) -> Self {
		Self {
			name,
			variant: variant.map(|i| quote! {::#i}),
			fields: vec![],
			unique,
			_marker: PhantomData,
		}
	}
}

impl StructBuilder<Named> {
	#[inline]
	pub fn field(mut self, res: &mut CodegenResult, name: Ident, value: &impl Codegen) -> Self {
		let val = res.add_value(value, self.unique);
		self.fields.push(quote!(#name: #val,));
		self
	}
	pub fn build(self, res: &mut CodegenResult) -> TokenStream {
		let name = &self.name;
		let variant = &self.variant;
		let fields = &self.fields;
		res.add_code(
			quote! {structdump_import::#name #variant {
				#(#fields)*
			}},
			Some(quote! {structdump_import::#name}),
			self.unique,
		)
	}
}

impl StructBuilder<Unnamed> {
	#[inline]
	pub fn field(mut self, res: &mut CodegenResult, value: &impl Codegen) -> Self {
		let val = res.add_value(value, self.unique);
		self.fields.push(quote!(#val,));
		self
	}
	pub fn build(self, res: &mut CodegenResult) -> TokenStream {
		let name = &self.name;
		let variant = &self.variant;
		let fields = &self.fields;
		res.add_code(
			quote! {structdump_import::#name #variant(
				#(#fields)*
			)},
			Some(quote! {structdump_import::#name}),
			self.unique,
		)
	}
}

impl StructBuilder<Unit> {
	pub fn build(self) -> TokenStream {
		let name = &self.name;
		let variant = &self.variant;
		quote! {structdump_import::#name #variant}
	}
}

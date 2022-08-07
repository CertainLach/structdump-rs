use proc_macro::TokenStream;
use quote::{format_ident, quote, quote_spanned};
use syn::spanned::Spanned;
use syn::{Data, Fields, Ident};

#[proc_macro_derive(Codegen)]
pub fn derive_codegen(input: TokenStream) -> TokenStream {
	let ast = match syn::parse(input) {
		Ok(v) => v,
		Err(e) => return e.to_compile_error().into(),
	};
	impl_codegen(&ast)
}
fn fields_codegen_match(fields: &Fields) -> proc_macro2::TokenStream {
	match fields {
		Fields::Named(ref fields) => {
			let f = fields.named.iter().map(|f| {
				let name = &f.ident;
				quote_spanned! {f.span()=>#name}
			});
			quote! {{#(#f, )*}}
		}
		Fields::Unnamed(ref fields) => {
			let f = fields.unnamed.iter().enumerate().map(|(i, f)| {
				let name = format_ident!("f{i}");
				quote_spanned! {f.span()=>#name}
			});
			quote! {(#(#f, )*)}
		}
		Fields::Unit => {
			quote! {}
		}
	}
}
fn fields_codegen_body(
	name: &Ident,
	variant: Option<&Ident>,
	fields: &Fields,
) -> proc_macro2::TokenStream {
	let name = name.to_string();
	let variant = variant
		.map(|v| {
			let v = v.to_string();
			quote! {
				Some(::structdump::format_ident!(#v))
			}
		})
		.unwrap_or_else(|| quote! {None});
	match fields {
		Fields::Named(ref fields) => {
			let f = fields.named.iter().map(|f| {
				let name = f
					.ident
					.clone()
					.expect("we're iterating over Fields::Named")
					.to_string();
				let ident = &f.ident;
				quote! {
					.field(res, ::structdump::format_ident!(#name), #ident)
				}
			});
			quote! {<::structdump::StructBuilder<::structdump::Named>>::new(::structdump::format_ident!(#name), #variant, unique)#(#f)*.build(res)}
		}
		Fields::Unnamed(ref fields) => {
			let f = fields.unnamed.iter().enumerate().map(|(i, _)| {
				let ident = format_ident!("f{i}");
				quote! {
					.field(res, #ident)
				}
			});
			quote! {<::structdump::StructBuilder<::structdump::Unnamed>>::new(::structdump::format_ident!(#name), #variant, unique)#(#f)*.build(res)}
		}
		Fields::Unit => {
			quote! {<::structdump::StructBuilder<::structdump::Unit>>::new(::structdump::format_ident!(#name), #variant, unique).build()}
		}
	}
}
fn impl_codegen(ast: &syn::DeriveInput) -> TokenStream {
	let name = &ast.ident;
	let out = match &ast.data {
		Data::Struct(ref data) => {
			let head = fields_codegen_match(&data.fields);
			let body = fields_codegen_body(name, None, &data.fields);
			quote! {
				let #name #head = &self;
				#body
			}
		}
		Data::Enum(data) => {
			let variants = data.variants.iter().map(|v| {
				let var_name = &v.ident;
				let match_q = fields_codegen_match(&v.fields);
				let match_b = fields_codegen_body(name, Some(var_name), &v.fields);
				quote_spanned! {v.span()=>
					#name::#var_name #match_q => {
						#match_b
					}
				}
			});
			quote! {
				match &self {
					#(#variants ,)*
				}
			}
		}
		Data::Union(_) => unimplemented!(),
	};
	let gen = quote! {
		impl ::structdump::Codegen for #name {
			fn gen_code(&self, res: &mut ::structdump::CodegenResult, unique: bool) -> ::structdump::TokenStream {
				#out
			}
		}
	};
	gen.into()
}

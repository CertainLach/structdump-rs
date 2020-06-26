use proc_macro::TokenStream;
use quote::{quote, quote_spanned};
use syn::spanned::Spanned;
use syn::{Data, Fields, Ident};

#[proc_macro_derive(Codegen)]
pub fn derive_codegen(input: TokenStream) -> TokenStream {
	let ast = syn::parse(input).unwrap();
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
				let name = Ident::new(&format!("f{}", i), syn::export::Span::call_site());
				quote_spanned! {f.span()=>#name}
			});
			quote! {(#(#f, )*)}
		}
		Fields::Unit => {
			quote! {}
		}
	}
}
fn fields_codegen_body(fields: &Fields) -> proc_macro2::TokenStream {
	match fields {
		Fields::Named(ref fields) => {
			let f = fields.named.iter().map(|f| {
				let name = &f.ident;
				quote_spanned! {f.span()=>{
					out.push_str(stringify!(#name));
					out.push(':');
					out.push_str(&res.add_value(&#name));
				}}
			});
			quote! {
				out.push('{');
				#(#f;
					out.push(',');
				)*
				out.push('}');
			}
		}
		Fields::Unnamed(ref fields) => {
			let f = fields.unnamed.iter().enumerate().map(|(i, f)| {
				let name = Ident::new(&format!("f{}", i), syn::export::Span::call_site());
				quote_spanned! {f.span()=>{
					out.push_str(&res.add_value(&#name));
				}}
			});
			quote! {
				out.push('(');
				#(#f;
					out.push(',');
				)*
				out.push(')');
			}
		}
		Fields::Unit => {
			quote! {}
		}
	}
}
fn impl_codegen(ast: &syn::DeriveInput) -> TokenStream {
	let name = &ast.ident;
	let out = match &ast.data {
		Data::Struct(ref data) => {
			let head = fields_codegen_match(&data.fields);
			let body = fields_codegen_body(&data.fields);
			quote! {
				let #name #head = &self;
				out.push_str(stringify!(#name));
				#body
			}
		},
		Data::Enum(data) => {
			let variants = data.variants.iter().map(|v| {
				let var_name = &v.ident;
				let match_q = fields_codegen_match(&v.fields);
				let match_b = fields_codegen_body(&v.fields);
				quote_spanned! {v.span()=>
					#name::#var_name #match_q => {
						out.push_str(stringify!(#var_name));
						#match_b
					}
				}
			});
			quote! {
				out.push_str(stringify!(#name));
				out.push_str("::");
				match self {
					#(#variants ,)*
				}
			}
		}
		Data::Union(_) => unimplemented!(),
	};
	let gen = quote! {
		impl ::codegen::Codegen for #name {
			fn gen_code(&self, res: &mut CodegenResult, out: &mut String) {
				#out
			}
		}
	};
	gen.into()
}

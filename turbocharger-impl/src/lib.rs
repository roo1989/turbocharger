//! This crate provides Turbocharger's procedural macros.
//!
//! Please refer to the `turbocharger` crate for how to set this up.

#![forbid(unsafe_code)]
#![allow(unused_imports)]

use proc_macro_error::{abort, abort_call_site, proc_macro_error};
use quote::{format_ident, quote, ToTokens};
use syn::{
 parse_macro_input, Data, DeriveInput, Expr, Fields, FieldsNamed, Ident, LitStr, Meta, NestedMeta,
 PathSegment, Token, Type,
};

/// Apply this to a function to make it available on the server only.
#[proc_macro_attribute]
#[proc_macro_error]
pub fn server_only(
 _args: proc_macro::TokenStream,
 input: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
 let orig_fn = parse_macro_input!(input as syn::ItemFn);

 let maybe_inject_once = if orig_fn.sig.ident.to_string() == "main" {
  quote! {
   #[cfg(target_arch = "wasm32")]
   #[allow(non_camel_case_types)]
   #[wasm_bindgen]
   pub struct wasm_only;

   #[cfg(target_arch = "wasm32")]
   #[allow(non_camel_case_types)]
   #[wasm_bindgen]
   pub struct backend;
  }
 } else {
  quote! {}
 };

 proc_macro::TokenStream::from(quote! {
  #maybe_inject_once

  #[cfg(not(target_arch = "wasm32"))]
  #[allow(dead_code)]
  #orig_fn
 })
}

/// Apply this to a `pub` `fn` to make it available to the WASM module only. Apples `#[wasm_bindgen]` underneath.
#[proc_macro_attribute]
#[proc_macro_error]
pub fn wasm_only(
 _args: proc_macro::TokenStream,
 input: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
 let orig_fn = parse_macro_input!(input as syn::ItemFn);
 proc_macro::TokenStream::from(quote! {
  #[cfg(target_arch = "wasm32")]
  #[wasm_bindgen(js_class = wasm_only)]
  impl wasm_only {
   #[wasm_bindgen]
   #orig_fn
  }
 })
}

/// Apply this to an `async fn` to make it available (over the network) to the JS frontend.
#[proc_macro_attribute]
#[proc_macro_error]
pub fn backend(
 _args: proc_macro::TokenStream,
 input: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
 let orig_fn = parse_macro_input!(input as syn::ItemFn);

 let orig_fn_ident = orig_fn.sig.ident.clone();
 let orig_fn_string = orig_fn_ident.to_string();
 let orig_fn_params = orig_fn.sig.inputs.clone();
 let orig_fn_sig_output = orig_fn.sig.output.clone();
 let output = orig_fn.sig.output.clone();
 let ret_ty = match output {
  syn::ReturnType::Default => None,
  syn::ReturnType::Type(_, path) => Some(path.clone()),
 };
 let ret_ty = match ret_ty.clone() {
  Some(path) => Some(*path),
  None => None,
 };
 let typepath = match ret_ty {
  Some(syn::Type::Path(typepath)) => Some(typepath),
  _ => None,
 };
 let orig_fn_ret_ty = match orig_fn_sig_output {
  syn::ReturnType::Default => None,
  syn::ReturnType::Type(_, path) => Some(*path),
 };

 // If return type is `Result<T, E>`, extract `T` so that `E` can be exchanged for `String` / `JsValue` for serialization and JS throws

 let path = match typepath {
  Some(syn::TypePath { path, .. }) => Some(path),
  _ => None,
 };
 let pair = match path {
  Some(syn::Path { mut segments, .. }) => segments.pop(),
  _ => None,
 };
 let pathsegment = match pair {
  Some(pair) => Some(pair.into_value()),
  _ => None,
 };
 let (is_result, arguments) = match pathsegment {
  Some(syn::PathSegment { ident, arguments }) => (ident == "Result", Some(arguments)),
  _ => (false, None),
 };

 let anglebracketedgenericarguments = match arguments {
  Some(syn::PathArguments::AngleBracketed(anglebracketedgenericarguments)) => {
   Some(anglebracketedgenericarguments)
  }
  _ => None,
 };

 let args = match anglebracketedgenericarguments {
  Some(syn::AngleBracketedGenericArguments { args, .. }) => Some(args),
  _ => None,
 };

 let result_inner_ty = match args {
  Some(args) => Some(args.into_iter().next()),
  _ => None,
 };

 let js_ret_ty = if is_result && result_inner_ty.is_some() {
  quote! { Result<#result_inner_ty, JsValue> }
 } else {
  quote! { #orig_fn_ret_ty }
 };

 let serialize_ret_ty = if is_result && result_inner_ty.is_some() {
  quote! { Result<#result_inner_ty, String> }
 } else {
  quote! { #orig_fn_ret_ty }
 };

 let maybe_map_err = if is_result && result_inner_ty.is_some() {
  quote! { .map_err(|e| e.to_string().into()) }
 } else {
  quote! {}
 };

 let tuple_indexes = (0..orig_fn_params.len()).map(|i| syn::Index::from(i));
 let orig_fn_param_names = orig_fn_params.iter().map(|p| match p {
  syn::FnArg::Receiver(_) => abort_call_site!("I don't know what to do with `self` here."),
  syn::FnArg::Typed(pattype) => match *pattype.pat.clone() {
   syn::Pat::Ident(i) => i.ident,
   _ => abort_call_site!("Parameter name is not Ident"),
  },
 });
 let orig_fn_param_names_cloned = orig_fn_param_names.clone();
 let orig_fn_param_tys = orig_fn_params.iter().map(|p| match p {
  syn::FnArg::Receiver(_) => abort_call_site!("I don't know what to do with `self` here."),
  syn::FnArg::Typed(pattype) => &pattype.ty,
 });
 let orig_fn_param_tys_cloned = orig_fn_param_tys.clone();

 let orig_fn_params_maybe_comma = if orig_fn_params.len() == 0 {
  quote! {}
 } else {
  quote! { , }
 };

 let mod_name = format_ident!("_TURBOCHARGER_{}", orig_fn_ident);
 let dispatch = format_ident!("_TURBOCHARGER_DISPATCH_{}", orig_fn_ident);
 let req = format_ident!("_TURBOCHARGER_REQ_{}", orig_fn_ident);
 let resp = format_ident!("_TURBOCHARGER_RESP_{}", orig_fn_ident);
 let impl_fn_ident = format_ident!("_TURBOCHARGER_IMPL_{}", orig_fn_ident);

 // let orig_fn_ret_ty = if orig_fn_ret_ty.is_some() {
 //  quote! { #orig_fn_ret_ty }
 // } else {
 //  quote! { () }
 // };

 proc_macro::TokenStream::from(quote! {
  #[cfg(not(target_arch = "wasm32"))]
  #orig_fn

  #[cfg(not(target_arch = "wasm32"))]
  #[allow(non_snake_case)]
  mod #mod_name {
   use ::turbocharger::typetag;
   #[::turbocharger::typetag::serde(name = #orig_fn_string)]
   #[::turbocharger::async_trait]
   impl ::turbocharger::RPC for super::#dispatch {
    async fn execute(&self) -> Vec<u8> {
     let response = super::#resp {
      txid: self.txid,
      result: super::#orig_fn_ident(#( self.params. #tuple_indexes .clone() ),*).await #maybe_map_err,
     };
     ::turbocharger::bincode::serialize(&response).unwrap()
    }
   }
  }

  #[cfg(target_arch = "wasm32")]
  #[wasm_bindgen]
  pub async fn #orig_fn_ident(#orig_fn_params) -> #js_ret_ty {
   #impl_fn_ident(#( #orig_fn_param_names ),*) .await #maybe_map_err
  }

  #[cfg(target_arch = "wasm32")]
  async fn #impl_fn_ident(#orig_fn_params) -> #serialize_ret_ty {
   {
    let tx = ::turbocharger::_Transaction::new();
    let req = ::turbocharger::bincode::serialize(&#req {
     typetag_const_one: 1,
     dispatch_name: #orig_fn_string,
     txid: tx.txid,
     params: (#( #orig_fn_param_names_cloned ),* #orig_fn_params_maybe_comma),
    })
    .unwrap();
    let response = tx.run(req).await;
    let #resp { result, .. } =
     ::turbocharger::bincode::deserialize(&response).unwrap();
    result
   }
  }

  #[allow(non_camel_case_types)]
  #[derive(::turbocharger::serde::Serialize, ::turbocharger::serde::Deserialize)]
  #[serde(crate = "::turbocharger::serde")]
  struct #req {
   typetag_const_one: i64,
   dispatch_name: &'static str,
   txid: i64,
   params: (#( #orig_fn_param_tys ),* #orig_fn_params_maybe_comma),
  }

  #[allow(non_camel_case_types)]
  #[derive(::turbocharger::serde::Serialize, ::turbocharger::serde::Deserialize, Debug)]
  #[serde(crate = "::turbocharger::serde")]
  struct #dispatch {
   txid: i64,
   params: (#( #orig_fn_param_tys_cloned ),* #orig_fn_params_maybe_comma),
  }

  #[allow(non_camel_case_types)]
  #[derive(::turbocharger::serde::Serialize, ::turbocharger::serde::Deserialize)]
  #[serde(crate = "::turbocharger::serde")]
  struct #resp {
   txid: i64,
   result: #serialize_ret_ty,
  }

 })
}

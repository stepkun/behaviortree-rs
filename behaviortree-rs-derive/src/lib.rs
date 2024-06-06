use std::collections::HashMap;

use proc_macro::TokenStream;
use proc_macro2::{Ident, Span};
use quote::ToTokens;
use syn::{
    parse::Parse, punctuated::Punctuated, token::Comma, visit_mut::{self, VisitMut}, AttrStyle, DeriveInput, FnArg, GenericParam, ImplItem, ImplItemFn, ItemImpl, ItemStruct, LitStr, Path, ReturnType, Type
};

#[macro_use]
extern crate quote;
#[macro_use]
extern crate syn;

extern crate proc_macro;

trait ToMap<T, K, V> {
    fn to_map(&self) -> syn::Result<std::collections::HashMap<K, V>>;
}

impl ToMap<Punctuated<syn::Meta, Comma>, syn::Ident, Option<proc_macro2::TokenStream>>
    for Punctuated<syn::Meta, Comma>
{
    /// Convert a list of attribute arguments to a HashMap
    fn to_map(
        &self,
    ) -> syn::Result<std::collections::HashMap<syn::Ident, Option<proc_macro2::TokenStream>>> {
        self.iter()
            .map(|m| {
                match m {
                    syn::Meta::NameValue(arg) => {
                        // Convert Expr to one of the valid types:
                        // Ident (variable name etc)
                        // ExprCall (function call etc)
                        // Lit (literal, for integer types etc)
                        if let syn::Expr::Lit(lit) = &arg.value {
                            if let syn::Lit::Str(arg_str) = &lit.lit {
                                let value = if let Ok(call) = arg_str.parse::<syn::ExprCall>() {
                                    quote! { #call }
                                }
                                else if let Ok(ident) = arg_str.parse::<syn::Ident>() {
                                    quote! { #ident }
                                }
                                else if let Ok(lit) = arg_str.parse::<syn::Lit>() {
                                    quote! { #lit }
                                }
                                else if let Ok(path) = arg_str.parse::<syn::Path>() {
                                    quote! { #path }
                                }
                                else {
                                    return Err(syn::Error::new_spanned(&arg.value, "argument value should be a:  variable, literal, path, function call"))
                                };

                                Ok((arg.path.get_ident().unwrap().clone(), Some(value)))
                            }
                            else {
                                Err(syn::Error::new_spanned(&arg.value, "argument value should be a string literal"))
                            }
                        }
                        else {
                            Err(syn::Error::new_spanned(&arg.value, "argument value should be a string literal"))
                        }
                    }
                    syn::Meta::Path(arg) => {
                        Ok((arg.get_ident().unwrap().clone(), None))
                    }
                    _ => Err(syn::Error::new_spanned(m, "argument type should be Path or NameValue: `#[bt(default)]`, or `#[bt(default = \"String::new()\")]`"))
                }
            })
            .collect()
    }
}

trait ConcatTokenStream {
    fn concat_list(&self, value: proc_macro2::TokenStream) -> proc_macro2::TokenStream;
    fn concat_blocks(&self, value: proc_macro2::TokenStream) -> proc_macro2::TokenStream;
}

impl ConcatTokenStream for proc_macro2::TokenStream {
    fn concat_list(&self, value: proc_macro2::TokenStream) -> proc_macro2::TokenStream {
        if self.is_empty() {
            if value.is_empty() {
                // Both are empty
                proc_macro2::TokenStream::new()
            } else {
                // self empty, value not empty
                quote! {
                    #value
                }
            }
        } else if value.is_empty() {
            // self not empty, value empty
            quote! {
                #self
            }
        } else {
            // Both have value
            quote! {
                #self,
                #value
            }
        }
    }

    fn concat_blocks(&self, value: proc_macro2::TokenStream) -> proc_macro2::TokenStream {
        if self.is_empty() {
            if value.is_empty() {
                // Both are empty
                proc_macro2::TokenStream::new()
            } else {
                // self empty, value not empty
                quote! {
                    #value
                }
            }
        } else if value.is_empty() {
            // self not empty, value empty
            quote! {
                #self
            }
        } else {
            // Both have value
            quote! {
                #self
                #value
            }
        }
    }
}

struct NodeAttribute {
    name: syn::Ident,
    value: syn::Ident,
}

impl Parse for NodeAttribute {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let name = input.parse()?;
        input.parse::<Token![=]>()?;
        let value = input.parse()?;

        Ok(Self {
            name, value
        })
    }
}

struct NodeImplConfig {
    node_type: syn::Ident,
    tick_fn: syn::Ident,
    on_start_fn: Option<syn::Ident>,
    ports: Option<syn::Ident>,
    halt: Option<syn::Ident>,
}

impl Parse for NodeImplConfig {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let node_type: Ident = input.parse()?;
        let node_type_str = node_type.to_string();

        if input.parse::<Token![,]>().is_ok() {
            let mut attributes: HashMap<String, NodeAttribute> = input.parse_terminated(NodeAttribute::parse, Token![,])?
                .into_iter()
                .map(|val| (val.name.to_string(), val))
                .collect();    
    
            let (tick_fn, on_start_fn) = if node_type_str == "StatefulActionNode" {
                let tick_fn = attributes
                    .remove("on_running")
                    .map(|val| val.value)
                    .unwrap_or_else(|| syn::parse2(quote! { on_running }).unwrap());
                
                let on_start_fn = attributes
                    .remove("on_start")
                    .map(|val| val.value)
                    .unwrap_or_else(|| syn::parse2(quote! { on_start }).unwrap());
    
                (tick_fn, Some(on_start_fn))
            } else {
                let tick_fn = attributes
                    .remove("tick")
                    .map(|val| val.value)
                    .unwrap_or_else(|| syn::parse2(quote! { tick }).unwrap());
    
                (tick_fn, None)
            };
    
            let ports = attributes.remove("ports").map(|val| val.value);
            let halt = attributes.remove("halt").map(|val| val.value);
    
            if let Some((_, invalid_field)) = attributes.into_iter().next() {
                return Err(syn::Error::new(invalid_field.name.span(), "invalid field name"));
            }

            Ok(Self {
                node_type,
                tick_fn,
                on_start_fn,
                ports,
                halt,
            })
        } else {
            let (tick_fn, on_start_fn) = if node_type_str == "StatefulActionNode" {
                (Ident::new("on_running", input.span()), Some(Ident::new("on_start", input.span())))
            } else {
                (Ident::new("tick", input.span()), None)
            };

            Ok(Self {
                node_type,
                tick_fn,
                on_start_fn,
                ports: None,
                halt: None,
            })
        }
    }
}

struct SelfVisitor;

impl VisitMut for SelfVisitor {
    fn visit_ident_mut(&mut self, i: &mut proc_macro2::Ident) {
        if i == "self" {
            let ctx = quote! { self_ };
            let ctx = syn::parse2(ctx).unwrap();
            
            *i = ctx;
        }

        visit_mut::visit_ident_mut(self, i)
    }
}

fn alter_node_fn(fn_item: &mut ImplItemFn, struct_type: &Type, is_async: bool) -> syn::Result<()> {
    // Remove async
    if is_async {
        fn_item.sig.asyncness = None;
    }
    // Add lifetime to signature
    let lifetime: GenericParam = syn::parse2(quote!{ 'a })?;
    fn_item.sig.generics.params.push(lifetime);
    // Rename parameters
    for arg in fn_item.sig.inputs.iter_mut() {
        if let FnArg::Receiver(_) = arg {
            let new_arg = quote! { node_: &'a mut ::behaviortree_rs::nodes::TreeNodeData };
            let new_arg = syn::parse2(new_arg)?;
            *arg = new_arg;
        }
    }

    let new_arg = syn::parse2(quote! { ctx: &'a mut ::std::boxed::Box<dyn ::core::any::Any + ::core::marker::Send + ::core::marker::Sync> })?;

    fn_item.sig.inputs.push(new_arg);

    let old_block = &mut fn_item.block;
    // Rename occurrences of self
    SelfVisitor.visit_block_mut(old_block);

    let new_block = if is_async {
        // Get old return without the -> token
        let old_return = match &fn_item.sig.output {
            ReturnType::Default => quote! { () },
            ReturnType::Type(_, ret) => quote! { #ret }
        };

        // Wrap return type in BoxFuture
        let new_return = quote! {
            -> ::futures::future::BoxFuture<'a, #old_return>
        };

        let new_return = syn::parse2(new_return)?;
        fn_item.sig.output = new_return;
    
        // Wrap function block in Box::pin and create ctx
        quote! {
            {
                ::std::boxed::Box::pin(async move {
                    let mut self_ = ctx.downcast_mut::<#struct_type>().unwrap();
                    #old_block
                })
            }
        }
    } else {
        // Wrap function block in Box::pin and create ctx
        quote! {
            {
                let mut self_ = ctx.downcast_mut::<#struct_type>().unwrap();
                #old_block
            }
        }
    };

    let new_block = syn::parse2(new_block)?;

    fn_item.block = new_block;

    Ok(())
}

fn bt_impl(
    mut args: NodeImplConfig,
    mut item: ItemImpl,
) -> syn::Result<proc_macro2::TokenStream> {
    let struct_type = &item.self_ty;
    
    for sub_item in item.items.iter_mut() {
        if let ImplItem::Fn(fn_item) = sub_item {
            let mut should_rewrite_def = false;
            // Rename methods
            let mut new_ident = None;
            // Check if it's a tick
            if fn_item.sig.ident == args.tick_fn {
                new_ident = if args.node_type == "StatefulActionNode" {
                    Some(syn::parse2(quote! { _on_running })?)
                } else {
                    Some(syn::parse2(quote! { _tick })?)
                };

                should_rewrite_def = true;
            }
            // Check if it's an on_start
            if let Some(on_start) = args.on_start_fn.as_ref() {
                if &fn_item.sig.ident == on_start {
                    new_ident = Some(syn::parse2(quote! { _on_start })?);
                    should_rewrite_def = true;
                }
            }
            // Check if it's a halt
            if let Some(halt) = args.halt.as_ref() {
                if &fn_item.sig.ident == halt {
                    new_ident = Some(syn::parse2(quote! { _halt })?);
                    should_rewrite_def = true;
                }
            } else if &fn_item.sig.ident == "halt" {
                args.halt = Some(fn_item.sig.ident.clone());
                new_ident = Some(syn::parse2(quote! { _halt })?);
                should_rewrite_def = true;
            }
            // Check if it's a ports
            if let Some(ports) = args.ports.as_ref() {
                if &fn_item.sig.ident == ports {
                    new_ident = Some(syn::parse2(quote! { _ports })?);
                }
            } else if &fn_item.sig.ident == "ports" {
                args.ports = Some(fn_item.sig.ident.clone());
                new_ident = Some(syn::parse2(quote! { _ports })?);
            }

            if let Some(new_ident) = new_ident {
                if should_rewrite_def {
                    alter_node_fn(fn_item, struct_type, true)?;
                }
                
                fn_item.sig.ident = new_ident;
            }
        }
    }

    let mut extra_impls = Vec::new();

    if args.halt.is_none() {
        extra_impls.push(syn::parse2(quote! {
            fn _halt<'a>(node_: &'a mut ::behaviortree_rs::nodes::TreeNodeData, ctx: &'a mut ::std::boxed::Box<dyn ::core::any::Any + ::core::marker::Send + ::core::marker::Sync>) -> ::futures::future::BoxFuture<'a, ()> { ::std::boxed::Box::pin(async move {}) }
        })?)
    }

    if args.ports.is_none() {
        extra_impls.push(syn::parse2(quote! {
            fn _ports() -> ::behaviortree_rs::basic_types::PortsList { ::behaviortree_rs::basic_types::PortsList::new() }
        })?)
    }

    item.items.extend(extra_impls);

    Ok(quote! { #item })
}

fn bt_struct(
    type_ident: Path,
    mut item: ItemStruct,
) -> syn::Result<proc_macro2::TokenStream> {
    let mut derives =
        vec![quote! { ::std::fmt::Debug }];

    let type_ident = type_ident.require_ident()?;
    let type_ident_str = type_ident.to_string();

    let item_ident = &item.ident;

    let mut default_fields = proc_macro2::TokenStream::new();
    let mut manual_fields = proc_macro2::TokenStream::new();
    let mut manual_fields_with_types = proc_macro2::TokenStream::new();
    let mut extra_impls = proc_macro2::TokenStream::new();

    match &mut item.fields {
        syn::Fields::Named(fields) => {
            for f in fields.named.iter_mut() {
                let name = f.ident.as_ref().unwrap();
                let ty = &f.ty;

                let mut used_default = false;
                for a in f.attrs.iter() {
                    if a.path().is_ident("bt") {
                        let args: Punctuated<syn::Meta, Comma> =
                            a.parse_args_with(Punctuated::parse_terminated)?;
                        let args_map = args.to_map()?;

                        // If the default argument was included
                        if let Some(value) = args_map.get(&syn::parse_str("default")?) {
                            used_default = true;
                            // Use the provided default, if provided by user
                            let default_value = if let Some(default_value) = value {
                                quote! { #default_value }
                            }
                            // Otherwise, use Default
                            else {
                                quote! { <#ty>::default() }
                            };

                            default_fields =
                                default_fields.concat_list(quote! { #name: #default_value });
                        }
                    }
                }

                // Mark field as manually specified if
                if !used_default {
                    manual_fields = manual_fields.concat_list(quote! { #name });
                    manual_fields_with_types =
                        manual_fields_with_types.concat_list(quote! { #name: #ty });
                }

                // Remove the bt attribute, keep all others
                f.attrs = f
                    .attrs
                    .clone()
                    .into_iter()
                    .filter(|a| !a.path().is_ident("bt"))
                    .collect();
            }
        }
        _ => {
            return Err(syn::Error::new_spanned(
                item,
                "expected a struct with named fields",
            ))
        }
    };

    let vis = &item.vis;
    let struct_fields = &item.fields;

    let mut user_attrs = Vec::new();

    for attr in item.attrs.iter() {
        if attr.path().is_ident("derive") {
            derives.push(attr.parse_args()?);
        } else if let AttrStyle::Outer = attr.style {
            user_attrs.push(attr);
        }
    }

    let user_attrs = user_attrs
        .into_iter()
        .fold(proc_macro2::TokenStream::new(), |acc, a| {
            // Only want to transfer outer attributes
            if let AttrStyle::Outer = a.style {
                if acc.is_empty() {
                    quote! {
                        #a
                    }
                } else {
                    quote! {
                        #acc
                        #a
                    }
                }
            } else {
                acc
            }
        });

    // Convert Vec of derive Paths into one TokenStream
    let derives = derives
        .into_iter()
        .fold(proc_macro2::TokenStream::new(), |acc, d| {
            if acc.is_empty() {
                quote! {
                    #d
                }
            } else {
                quote! {
                    #acc, #d
                }
            }
        });

    let extra_fields = proc_macro2::TokenStream::new()
        .concat_list(default_fields)
        .concat_list(manual_fields);

    // let node_type = match type_ident_str.as_str() {
    //     ""
    // }

    // let node_type = if type_ident == "StatefulActionNode" || type_ident == "SyncActionNode" {
    //     syn::parse2::<Ident>(quote! { ActionNode })?
    // } else {
    //     type_ident.clone()
    // };

    let node_category = match type_ident_str.as_str() {
        "StatefulActionNode" | "SyncActionNode" => syn::parse2::<Path>(quote! { Action })?,
        "ControlNode" => syn::parse2::<Path>(quote! { Control })?,
        "DecoratorNode" => syn::parse2::<Path>(quote! { Decorator })?,
        _ => return Err(syn::Error::new_spanned(type_ident, "Invalid node type"))
    };

    let node_type = match type_ident_str.as_str() {
        "StatefulActionNode" => syn::parse2::<Path>(quote! { StatefulAction })?,
        "SyncActionNode" => syn::parse2::<Path>(quote! { SyncAction })?,
        "ControlNode" => syn::parse2::<Path>(quote! { Control })?,
        "DecoratorNode" => syn::parse2::<Path>(quote! { Decorator })?,
        _ => return Err(syn::Error::new_spanned(type_ident, "Invalid node type"))
    };

    let node_specific_tokens = node_fields(&type_ident_str);

    let struct_name = LitStr::new(&item_ident.to_token_stream().to_string(), item_ident.span());

    let output = quote! {
        #user_attrs
        #[derive(#derives)]
        #vis struct #item_ident #struct_fields

        impl #item_ident {
            pub fn create_node(name: impl AsRef<str>, config: ::behaviortree_rs::nodes::NodeConfig, #manual_fields_with_types) -> ::behaviortree_rs::nodes::TreeNode {
                let ctx = #item_ident {
                    #extra_fields
                };

                let node_data = ::behaviortree_rs::nodes::TreeNodeData {
                    name: name.as_ref().to_string(),
                    type_str: String::from(#struct_name),
                    node_type: ::behaviortree_rs::nodes::NodeType::#node_type,
                    node_category: ::behaviortree_rs::basic_types::NodeCategory::#node_category,
                    config,
                    status: ::behaviortree_rs::basic_types::NodeStatus::Idle,
                    children: ::std::vec::Vec::new(),
                    ports_fn: Self::_ports,
                };
                
                ::behaviortree_rs::nodes::TreeNode {
                    data: node_data,
                    context: ::std::boxed::Box::new(ctx),
                    halt_fn: Self::_halt,
                    #node_specific_tokens
                }
            }
        }

        #extra_impls
    };

    Ok(output)
}

fn node_fields(type_ident_str: &str) -> proc_macro2::TokenStream {
    match type_ident_str {
        "StatefulActionNode" => {
            quote! {
                tick_fn: Self::_on_running,
                start_fn: Self::_on_start,
            }
        }
        // Don't need to check others, it has already been checked before now
        _ => {
            quote! {
                tick_fn: Self::_tick,
                start_fn: Self::_tick,
            }
        }
    }
}

/// Macro used to automatically generate the default boilerplate needed for all `TreeNode`s.
///
/// # Basic Usage
///
/// To use the macro, you need to add `#[bt_node(...)]` above your struct. As an argument
/// to the attribute, specify the NodeType that you would like to implement.
///
/// Supported options:
/// - `SyncActionNode`
/// - `StatefulActionNode`
/// - `ControlNode`
/// - `DecoratorNode`
///
/// By default, the tick method implementation is `async`. To specify this explicitly (or
/// make it synchronous), add `Async` or `Sync` after the node type.
///
/// ===
///
/// ```rust
/// use behaviortree_rs::{bt_node, basic_types::NodeStatus, nodes::{AsyncTick, NodeResult, AsyncHalt, NodePorts}, sync::BoxFuture};
///
/// // Here we are specifying a `SyncActionNode` as the node type.
/// #[bt_node(SyncActionNode)]
/// // Defaults to #[bt_node(SyncActionNode, Async)]
/// struct MyActionNode {} // No additional fields
///
/// // Now I need to `impl TreeNode`
/// impl AsyncTick for MyActionNode {
///     fn tick(&mut self) -> BoxFuture<NodeResult> {
///         Box::pin(async move {
///             // Do something here
///             // ...
///
///             Ok(NodeStatus::Success)
///         })
///     }
/// }
///
/// impl NodePorts for MyActionNode {}
///
/// // Also need to `impl NodeHalt`
/// // However, we'll just use the default implementation
/// impl AsyncHalt for MyActionNode {}
/// ```
///
/// ===
///
/// The above code will add fields to `MyActionNode` and create a `new()` associated method:
///
/// ```ignore
/// impl DummyActionNode {
///     pub fn new(name: impl AsRef<str>, config: NodeConfig) -> DummyActionNode {
///         Self {
///             name: name.as_ref().to_string(),
///             config,
///             status: NodeStatus::Idle
///         }
///     }
/// }
/// ```
///
/// # Adding Fields
///
/// When you add your own fields into the struct, be default they will be added
/// to the `new()` definition as arguments. To specify default values, use
/// the `#[bt(default)]` attribute above the fields.
///
/// `#[bt(default)]` will use the type's implementation of the `Default` trait. If
/// the trait isn't implemented on the type, or if you want to manually specify
/// a value, use `#[bt(default = "...")]`, where `...` is the value.
///
/// Valid argument types within the `"..."` are:
///
/// ```ignore
/// // Function calls
/// #[bt(default = "String::from(10)")]
///
/// // Variables
/// #[bt(default = "foo")]
///
/// // Paths (like enums)
/// #[bt(default = "NodeStatus::Idle")]
///
/// // Literals
/// #[bt(default = "10")]
/// ```
///
/// ## Example
///
/// ```rust
/// use behaviortree_rs::{bt_node, basic_types::NodeStatus, nodes::{AsyncTick, NodePorts, NodeResult, AsyncHalt}, sync::BoxFuture};
///
/// #[bt_node(SyncActionNode)]
/// struct MyActionNode {
///     #[bt(default = "NodeStatus::Success")]
///     foo: NodeStatus,
///     #[bt(default)] // defaults to empty String
///     bar: String
/// }
///
/// // Now I need to `impl TreeNode`
/// impl AsyncTick for MyActionNode {
///     fn tick(&mut self) -> BoxFuture<NodeResult> {
///         Box::pin(async move {
///             Ok(NodeStatus::Success)
///         })
///     }
/// }
///
/// impl NodePorts for MyActionNode {}
///
/// impl AsyncHalt for MyActionNode {}
/// ```
#[proc_macro_attribute]
pub fn bt_node(args: TokenStream, input: TokenStream) -> TokenStream {
    if let Ok(struct_) = syn::parse::<ItemStruct>(input.clone()) {
        let args = parse_macro_input!(args as Path);
        // let args = parse_macro_input!(args as NodeStructConfig);
        bt_struct(args, struct_).unwrap_or_else(syn::Error::into_compile_error).into()
    } else if let Ok(impl_) = syn::parse::<ItemImpl>(input) {
        let args = parse_macro_input!(args as NodeImplConfig);
        bt_impl(args, impl_).unwrap_or_else(syn::Error::into_compile_error).into()
    } else {
        syn::Error::new(Span::call_site(), "The `bt_node` macro must be used on either a `struct` or `impl` block.").into_compile_error().into()
    }

    // let args_parsed = parse_macro_input!(args as NodeStructConfig);
    // let item = parse_macro_input!(input as ItemStruct);

    // bt_struct(args_parsed, item)
    //     .unwrap_or_else(syn::Error::into_compile_error)
    //     .into()
}

#[proc_macro_derive(FromString)]
pub fn derive_from_string(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    let ident = input.ident;

    let expanded = quote! {
        impl ::behaviortree_rs::basic_types::FromString for #ident {
            type Err = <#ident as ::core::str::FromStr>::Err;

            fn from_string(value: impl AsRef<str>) -> Result<#ident, Self::Err> {
                value.as_ref().parse()
            }
        }
    };

    TokenStream::from(expanded)
}

#[proc_macro_derive(BTToString)]
pub fn derive_bt_to_string(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    let ident = input.ident;

    let expanded = quote! {
        impl ::behaviortree_rs::basic_types::BTToString for #ident {
            fn bt_to_string(&self) -> String {
                ::std::string::ToString::to_string(self)
            }
        }
    };

    TokenStream::from(expanded)
}

struct NodeRegistration {
    factory: syn::Ident,
    name: proc_macro2::TokenStream,
    node_type: syn::Type,
    params: Punctuated<syn::Expr, Comma>,
}

impl Parse for NodeRegistration {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let factory = input.parse()?;
        input.parse::<Token![,]>()?;
        
        let node_name = input.parse::<syn::Expr>()?.to_token_stream();
        
        input.parse::<Token![,]>()?;
        let node_type = input.parse()?;
        // If there are extra parameters, try to parse a comma. Otherwise skip
        if !input.is_empty() {
            input.parse::<Token![,]>()?;
        }
    
        let params = input.parse_terminated(syn::Expr::parse, Token![,])?;

        Ok(Self {
            factory,
            name: node_name,
            node_type,
            params,
        })
    }
}

fn build_node(node: &NodeRegistration) -> proc_macro2::TokenStream {
    let NodeRegistration {
        factory: _,
        name,
        node_type,
        params
    } = node;

    let cloned_names = (0..params.len())
        .fold(quote!{}, |acc, i| {
            let arg_name = Ident::new(&format!("arg{i}"), Span::call_site());
            quote!{ #acc, #arg_name.clone() }
        });

    quote! {
        {
            let mut node = #node_type::create_node(#name, config #cloned_names);
            let manifest = ::behaviortree_rs::basic_types::TreeNodeManifest {
                node_type: node.node_category(),
                registration_id: #name.into(),
                ports: node.provided_ports(),
                description: ::std::string::String::new(),
            };
            node.config_mut().set_manifest(::std::sync::Arc::new(manifest));
            node
        }
    }
}

fn register_node(input: TokenStream, node_type_token: proc_macro2::TokenStream, node_type: NodeTypeInternal) -> TokenStream {
    let node_registration = parse_macro_input!(input as NodeRegistration);

    let factory = &node_registration.factory;
    let name = &node_registration.name;
    let params = &node_registration.params;

    // Create expression that clones all parameters
    let param_clone_expr = params
        .iter()
        .enumerate()
        .fold(quote!{}, |acc, (i, item)| {
            let arg_name = Ident::new(&format!("arg{i}"), Span::call_site());
            quote! {
                #acc
                let #arg_name = #item.clone();
            }
        });

    let node = build_node(&node_registration);

    let extra_steps = match node_type {
        NodeTypeInternal::Control => quote! {
            node.data.children = children;
        },
        NodeTypeInternal::Decorator => quote! { 
            node.data.children = children;
        },
        _ => quote!{ }
    };

    let expanded = quote! {
        {
            let blackboard = #factory.blackboard().clone();

            #param_clone_expr

            let node_fn = move |
                config: ::behaviortree_rs::nodes::NodeConfig,
                mut children: ::std::vec::Vec<::behaviortree_rs::nodes::TreeNode>
            | -> ::behaviortree_rs::nodes::TreeNode
            {
                let mut node = #node;
                
                #extra_steps

                node
            };

            #factory.register_node(#name, node_fn, #node_type_token);
        }
    };

    TokenStream::from(expanded)
}

enum NodeTypeInternal {
    Action,
    Control,
    Decorator,
}

/// Registers an Action type node with the factory.
/// 
/// **NOTE:** During tree creation, a new node is created using the parameters
/// given after the node type field. You specified these fields in your node struct
/// definition. Each time a node is created, the parameters are cloned using `Clone::clone`.
/// Thus, your parameters must implement `Clone`.
/// 
/// # Usage
/// 
/// ```ignore
/// let mut factory = Factory::new();
/// let arg1 = String::from("hello world");
/// let arg2 = 10u32;
/// 
/// register_action_node!(factory, "TestNode", TestNode, arg1, arg2);
/// ```
#[proc_macro]
pub fn register_action_node(input: TokenStream) -> TokenStream {
    register_node(input, quote! { ::behaviortree_rs::basic_types::NodeCategory::Action }, NodeTypeInternal::Action)
}

/// Registers an Control type node with the factory.
/// 
/// **NOTE:** During tree creation, a new node is created using the parameters
/// given after the node type field. You specified these fields in your node struct
/// definition. Each time a node is created, the parameters are cloned using `Clone::clone`.
/// Thus, your parameters must implement `Clone`.
/// 
/// # Usage
/// 
/// ```ignore
/// let mut factory = Factory::new();
/// let arg1 = String::from("hello world");
/// let arg2 = 10u32;
/// 
/// register_control_node!(factory, "TestNode", TestNode, arg1, arg2);
/// ```
#[proc_macro]
pub fn register_control_node(input: TokenStream) -> TokenStream {
    register_node(input, quote! { ::behaviortree_rs::basic_types::NodeCategory::Control }, NodeTypeInternal::Control)
}

/// Registers an Decorator type node with the factory.
/// 
/// **NOTE:** During tree creation, a new node is created using the parameters
/// given after the node type field. You specified these fields in your node struct
/// definition. Each time a node is created, the parameters are cloned using `Clone::clone`.
/// Thus, your parameters must implement `Clone`.
/// 
/// # Usage
/// 
/// ```ignore
/// let mut factory = Factory::new();
/// let arg1 = String::from("hello world");
/// let arg2 = 10u32;
/// 
/// register_decorator_node!(factory, "TestNode", TestNode, arg1, arg2);
/// ```
#[proc_macro]
pub fn register_decorator_node(input: TokenStream) -> TokenStream {
    register_node(input, quote! { ::behaviortree_rs::basic_types::NodeCategory::Decorator }, NodeTypeInternal::Decorator)
}

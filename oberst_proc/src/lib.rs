use std::{collections::HashMap, error};

use proc_macro::TokenStream;
use quote::quote;
use syn::{braced, bracketed, parse_macro_input, spanned::Spanned, Ident, ItemFn, Type};

#[proc_macro]
pub fn define_command(input: TokenStream) -> TokenStream {
    let CommandDefiniton {
        name,
        context_type,
        variants,
    } = parse_macro_input!(input as CommandDefiniton);

    let functions = variants.iter().map(|variant| &variant.function);

    let dispatchers = variants.iter().map(|variant| {
        let name = variant.function.sig.ident.clone();
        let parser = variant.syntax.iter().map(|syntax| match syntax {
            CommandSyntax::Literal(literal) => {
                quote! {
                    parser.lit(#literal)?;
                }
            }
            CommandSyntax::Argument(name, ty) => {
                quote! {
                    let #name = parser.argument::<#ty>()?;
                }
            }
        });

        let call_args = variant.syntax.iter().filter_map(|item| match item {
            CommandSyntax::Literal(_) => None,
            CommandSyntax::Argument(name, _) => Some(name),
        });

        quote! {
            CommandDispatch {
                parser: |parser| {
                    #(#parser)*
                    Ok(Box::new(move |ctx| {
                        #name(ctx, #(#call_args,)*)
                    }))
                },
            }
        }
    });

    let usages = variants.iter().map(|variant| &variant.usage);

    let result = quote! {
        mod #name {
            use super::*;
            pub static DISPATCHERS: &[CommandDispatch<#context_type>] = &[
                #(#dispatchers),*
            ];

            pub static USAGE: CommandUsage = CommandUsage {
                name: stringify!(#name),
                usage: &[
                    #(
                        #usages,
                    )*
                ],
                description: None,
            };

            #(#functions)*
        }
    };
    // panic!("{}", result.to_string());
    result.into()
}

struct CommandDefiniton {
    name: Ident,
    context_type: Type,
    variants: Vec<CommandVariant>,
}

impl syn::parse::Parse for CommandDefiniton {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let name = input.parse()?;
        input.parse::<syn::Token![:]>()?;
        let context_type = input.parse()?;
        let variant_block;
        braced!(variant_block in input);
        let mut variants = vec![];
        while !variant_block.is_empty() {
            let func = variant_block.parse::<syn::ItemFn>()?;
            let arg_type_map = func
                .sig
                .inputs
                .iter()
                .map(|arg| {
                    if let syn::FnArg::Typed(pat) = arg {
                        if let syn::Pat::Ident(ident) = &*pat.pat {
                            Ok((ident.ident.clone(), *pat.ty.clone()))
                        } else {
                            return Err(syn::Error::new(pat.pat.span(), "Expected identifier"));
                        }
                    } else {
                        return Err(syn::Error::new(arg.span(), "Expected typed argument"));
                    }
                })
                .collect::<syn::Result<Vec<_>>>()?;

            let syntax = arg_type_map
                .iter()
                .skip(1)
                .map(|(arg_name, ty)| CommandSyntax::Argument(arg_name.clone(), ty.clone()))
                .collect::<Vec<_>>();
            variants.push(CommandVariant {
                function: func,
                usage: build_usage_string(&syntax),
                syntax,
            });
        }
        Ok(Self {
            name,
            context_type,
            variants,
        })
    }
}

struct CommandVariant {
    function: ItemFn,
    usage: String,
    syntax: Vec<CommandSyntax>,
}

enum CommandSyntax {
    Literal(String),
    Argument(Ident, Type),
}

fn build_usage_string(syntax: &[CommandSyntax]) -> String {
    syntax
        .iter()
        .map(|s| match s {
            CommandSyntax::Literal(lit) => lit.to_string(),
            CommandSyntax::Argument(name, ty) => format!("<{}: {}>", name, quote! { #ty }),
        })
        .collect::<Vec<_>>()
        .join(" ")
}

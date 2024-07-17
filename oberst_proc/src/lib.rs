use std::collections::HashMap;

use proc_macro::{Span, TokenStream};
use quote::{quote, ToTokens};
use syn::{
    braced, parenthesized, parse_macro_input, parse_quote, spanned::Spanned, Attribute, Error,
    FnArg, Ident, ItemFn, Pat, PatType, Signature, Type, TypeReference,
};

#[proc_macro]
pub fn define_command(input: TokenStream) -> TokenStream {
    let CommandDefiniton {
        name,
        context_type,
        variants,
    } = parse_macro_input!(input as CommandDefiniton);

    let functions = variants.iter().map(|variant| &variant.function);

    let dispatchers = variants.iter().map(|variant| {
        let parser = variant.generate_parser();

        quote! {
            CommandDispatch {
                parser: #parser,
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
        let context_type;
        parenthesized!(context_type in input);
        let context_type: Type = context_type.parse()?;

        let variant_block;
        braced!(variant_block in input);
        let mut variants = vec![];
        while !variant_block.is_empty() {
            let mut function = variant_block.parse::<syn::ItemFn>()?;
            check_context_arg(&function.sig, &context_type)?;
            let arg_names = extract_args_from_signature(&function.sig)?;

            let syntax =
                if let Some(usage) = extract_usage_string_from_metadata(&mut function.attrs)? {
                    build_syntax_from_usage(&arg_names, usage)
                } else {
                    build_syntax_from_signature(&arg_names)
                };

            variants.push(CommandVariant {
                function,
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

impl CommandVariant {
    fn generate_caller(&self) -> syn::Expr {
        let args = self.syntax.iter().filter_map(|syntax| match syntax {
            CommandSyntax::Literal(_) => None,
            CommandSyntax::Argument(name, _) => Some(name),
        });

        let return_type = &self.function.sig.output;
        let name = &self.function.sig.ident;

        let call: syn::Block = if let syn::ReturnType::Default = return_type {
            parse_quote! { {
                #name(ctx, #(#args,)*);
                Ok(0)
            }
            }
        } else {
            parse_quote! {
               { #name(ctx, #(#args,)*) }
            }
        };

        parse_quote! {
            Ok(Box::new(|ctx| {
                #call
            }))
        }
    }

    fn generate_parser(&self) -> syn::Expr {
        let parser = self.syntax.iter().map(|syntax| match syntax {
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

        let caller = self.generate_caller();
        parse_quote! {
            |parser| {
                #(#parser)*
                #caller

            }
        }
    }
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

fn extract_args_from_signature(sig: &Signature) -> syn::Result<HashMap<Ident, Type>> {
    sig.inputs
        .iter()
        .skip(1)
        .map(|arg| {
            if let FnArg::Typed(pat) = arg {
                if let Pat::Ident(ident) = &*pat.pat {
                    Ok((ident.ident.clone(), *pat.ty.clone()))
                } else {
                    return Err(Error::new(pat.pat.span(), "Expected identifier"));
                }
            } else {
                return Err(Error::new(arg.span(), "Expected typed argument"));
            }
        })
        .collect::<syn::Result<HashMap<_, _>>>()
}

fn extract_usage_string_from_metadata(attrs: &mut Vec<Attribute>) -> syn::Result<Option<String>> {
    let mut usage = None;

    for (i, attr) in attrs.iter().enumerate() {
        if attr.path().is_ident("usage") {
            match &attr.meta {
                syn::Meta::NameValue(syn::MetaNameValue {
                    value:
                        syn::Expr::Lit(syn::ExprLit {
                            lit: syn::Lit::Str(lit),
                            ..
                        }),
                    ..
                }) => {
                    usage = Some(lit.value());
                    attrs.remove(i);
                    break;
                }
                _ => {
                    return Err(Error::new(attr.span(), "Expected usage string"));
                }
            }
        }
    }
    Ok(usage)
}

fn build_syntax_from_signature(arg_names: &HashMap<Ident, Type>) -> Vec<CommandSyntax> {
    arg_names
        .iter()
        .map(|(name, ty)| CommandSyntax::Argument(name.clone(), ty.clone()))
        .collect()
}

fn build_syntax_from_usage(arg_names: &HashMap<Ident, Type>, usage: String) -> Vec<CommandSyntax> {
    return usage
        .split(" ")
        .into_iter()
        .map(|segment| {
            if segment.starts_with("<") {
                let name = segment
                    .chars()
                    .skip(1)
                    .take_while(|c| *c != '>')
                    .collect::<String>();
                let name_ident = Ident::new(&name, Span::call_site().into());
                let ty = arg_names.get(&name_ident);
                if let Some(ty) = ty {
                    CommandSyntax::Argument(name_ident, ty.clone())
                } else {
                    panic!("Unknown argument: {}", name);
                }
            } else {
                CommandSyntax::Literal(segment.to_string())
            }
        })
        .collect();
}

fn check_context_arg(sig: &Signature, context_type: &Type) -> syn::Result<()> {
    match sig.inputs.first() {
        Some(FnArg::Typed(PatType { ty, .. })) => match &**ty {
            Type::Reference(TypeReference { elem, .. })
                if elem.to_token_stream().to_string()
                    == context_type.to_token_stream().to_string() =>
            // Since syn::Type doesn't implement PartialEq, we have to convert to a string
            {
                Ok(())
            }
            _ => Err(Error::new(ty.span(), "Expected reference to context type")),
        },
        _ => Err(Error::new(
            sig.inputs.span(),
            "Expected reference to context type",
        )),
    }
}

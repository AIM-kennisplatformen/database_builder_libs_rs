use syn::{LitStr, parse::Parse, parse::ParseStream};

pub(crate) struct ModelArgs {
    pub(crate) kind: Option<String>,
}

impl Parse for ModelArgs {
    fn parse(input: ParseStream<'_>) -> syn::Result<Self> {
        if input.is_empty() {
            return Ok(Self { kind: None });
        }

        let kind: syn::Ident = input.parse()?;
        if !input.is_empty() {
            return Err(input.error("unexpected tokens after model kind"));
        }
        Ok(Self {
            kind: Some(kind.to_string()),
        })
    }
}

pub(crate) struct TypeDbNameArgs {
    pub(crate) name: Option<LitStr>,
}

impl Parse for TypeDbNameArgs {
    fn parse(input: ParseStream<'_>) -> syn::Result<Self> {
        if input.is_empty() {
            return Ok(Self { name: None });
        }

        let name = if input.peek(LitStr) {
            input.parse()?
        } else {
            let ident: syn::Ident = input.parse()?;
            if ident != "name" {
                return Err(syn::Error::new(ident.span(), "expected `name`"));
            }
            input.parse::<syn::Token![=]>()?;
            input.parse()?
        };

        if !input.is_empty() {
            return Err(input.error("unexpected tokens after TypeDB name"));
        }

        Ok(Self { name: Some(name) })
    }
}

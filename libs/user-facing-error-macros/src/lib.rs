extern crate proc_macro;

use darling::FromDeriveInput;
use once_cell::sync::Lazy;
use proc_macro::TokenStream;
use proc_macro2::{Ident, Span};
use quote::quote;
use regex::Regex;
use std::collections::BTreeSet;
use syn::DeriveInput;

#[proc_macro_derive(UserFacingError, attributes(user_facing))]
pub fn derive_user_facing_error(input: TokenStream) -> TokenStream {
    let input = syn::parse_macro_input!(input as DeriveInput);
    let input = UserErrorDeriveInput::from_derive_input(&input);

    if let Err(err) = input.as_ref() {
        panic!("{}", err);
    }

    let input = input.unwrap();

    let ident = &input.ident;
    let error_code = input.code.as_str();
    let message_template = input.message;
    let template_variables = message_template_variables(message_template.value().as_str(), &message_template.span());

    // Transform from the spec string templates with `${var}` to a rust format string we can use
    // with `format!()`.
    let message_template = message_template.value().replace("${", "{");

    let template_variables = template_variables.iter();

    let output = quote! {
        impl crate::UserFacingError for #ident {
            const ERROR_CODE: &'static str = #error_code;

            fn message(&self) -> String {
                format!(
                    #message_template,
                    #(
                        #template_variables = self.#template_variables
                    ),*
                )
            }
        }
    };

    output.into()
}

#[derive(Debug, FromDeriveInput)]
#[darling(attributes(user_facing))]
struct UserErrorDeriveInput {
    /// The name of the struct.
    ident: syn::Ident,
    /// The error code.
    code: String,
    /// The error message format string.
    message: syn::LitStr,
}

/// See MESSAGE_VARIABLE_REGEX
const MESSAGE_VARIABLE_REGEX_PATTERN: &str = r##"(?x)
    \$\{  # A curly brace preceded by a dollar sign
    (
        [a-zA-Z0-9_]+  # any number of alphanumeric characters and underscores
    )
    }  # a closing curly brace
"##;

/// The regex for variables in message templates.
static MESSAGE_VARIABLE_REGEX: Lazy<Regex> = Lazy::new(|| Regex::new(MESSAGE_VARIABLE_REGEX_PATTERN).unwrap());

fn message_template_variables(template: &str, span: &Span) -> BTreeSet<Ident> {
    let captures = MESSAGE_VARIABLE_REGEX.captures_iter(&template);

    captures
        // The unwrap is safe because we know this regex has one capture group.
        .map(|capture| capture.get(1).unwrap())
        .map(|m| Ident::new(m.as_str(), span.clone()))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn new_span() -> proc_macro2::Span {
        proc_macro2::Span::call_site()
    }

    fn assert_template_variables(template: &str, expected: &[&str]) {
        let result: Vec<String> = message_template_variables(template, &new_span())
            .iter()
            .map(|ident| ident.to_string())
            .collect();

        assert_eq!(result, expected);
    }

    #[test]
    fn message_template_variables_works() {
        assert_template_variables("no variables", &[]);
        assert_template_variables("${abc}_def", &["abc"]);
        assert_template_variables("abc${_def}", &["_def"]);
        assert_template_variables("some ${ code } sample", &[] as &[&str]);
        assert_template_variables("positional parameter ${} ", &[] as &[&str]);
        assert_template_variables(
            "Message with ${multiple_variables} to ${substitute}",
            &["multiple_variables", "substitute"],
        );
    }
}

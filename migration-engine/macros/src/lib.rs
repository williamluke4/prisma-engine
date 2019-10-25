extern crate proc_macro;

use darling::FromMeta;
use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, AttributeArgs, ItemFn};

// #[derive(Debug, FromMeta)]
// struct TestOneConnectorArgs {
//     /// The name of the connector to test.
//     #[darling(default)]
//     connector: String,
// }

#[derive(Debug, FromMeta)]
struct TestEachConnectorArgs {
    /// Comma-separated list of connectors to exclude.
    #[darling(default)]
    ignore: Option<String>,
}

#[proc_macro_attribute]
pub fn test_each_connector(attr: TokenStream, input: TokenStream) -> TokenStream {
    let attributes_meta: syn::AttributeArgs = parse_macro_input!(attr as AttributeArgs);
    let args = TestEachConnectorArgs::from_list(&attributes_meta);

    match args {
        Ok(_) => (),
        Err(err) => panic!("{}", err),
    };

    let mut test_function = parse_macro_input!(input as ItemFn);

    if test_function.sig.asyncness.is_none() {
        panic!("#[test_each_connector] works only with async test functions.");
    }

    let test_fn_name = test_function.sig.ident;
    let test_fn_impl_name = syn::Ident::new(&format!("__impl_{}", &test_fn_name), proc_macro2::Span::call_site());

    test_function.sig.ident = test_fn_impl_name.clone();

    // assert_eq!(
    //     test_function.sig.inputs.len(),
    //     1,
    //     "An async test with #[test_each_connector] only takes one argument."
    // );

    let setup = quote! {
        #[test]
        fn #test_fn_name() {
            use futures::future::FutureExt;

            test_each_connector(|a, b| #test_fn_impl_name(a, b).boxed())
        }

        #test_function
    };

    setup.into()
}

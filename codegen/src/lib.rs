#[cfg(feature = "es")]
use arcana_codegen_impl::es;
use arcana_codegen_impl::expand_derive;
use proc_macro::TokenStream;
use proc_macro_error::proc_macro_error;

#[cfg(feature = "es")]
#[proc_macro_error]
#[proc_macro_derive(Event, attributes(event))]
pub fn event_derive(input: TokenStream) -> TokenStream {
    expand_derive(input, es::derive_event)
}

pub use static_assertions as sa;

pub mod unique_event_type_and_ver {

    #[doc(hidden)]
    #[macro_export]
    macro_rules! unique_event_type_and_ver_for_struct {
        ($max_events: literal, $event_type: literal, $event_ver: literal) => {
            pub const fn __arcana_event_types() -> [Option<(&'static str, u16)>; $max_events] {
                let mut res = [None; $max_events];
                res[0] = Some(($event_type, $event_ver));
                res
            }
        };
    }

    #[doc(hidden)]
    #[macro_export]
    macro_rules! unique_event_type_and_ver_for_enum {
        ($max_events: literal, $($event_type: ty),* $(,)?) => {
            pub const fn __arcana_event_types() ->
                [Option<(&'static str, u16)>; $max_events]
            {
                let mut res = [None; $max_events];

                let mut global = 0;

                $({
                    let ev = <$event_type>::__arcana_event_types();
                    let mut local = 0;
                    while let Some(s) = ev[local] {
                        res[global] = Some(s);
                        local += 1;
                        global += 1;
                    }
                })*

                res
            }
        };
    }

    #[doc(hidden)]
    #[macro_export]
    macro_rules! unique_event_type_and_ver_check {
        ($event: ty) => {
            $crate::private::sa::const_assert!(
                $crate::private::unique_event_type_and_ver::all_unique(
                    <$event>::__arcana_event_types()
                )
            );
        };
    }

    #[doc(hidden)]
    #[must_use]
    pub const fn all_unique<const N: usize>(types: [Option<(&str, u16)>; N]) -> bool {
        const fn str_eq(l: &str, r: &str) -> bool {
            let (l, r) = (l.as_bytes(), r.as_bytes());

            if l.len() != r.len() {
                return false;
            }

            let mut i = 0;
            while i < l.len() {
                if l[i] != r[i] {
                    return false;
                }
                i += 1;
            }

            true
        }

        let mut outer = 0;
        while let Some((outer_type, outer_ver)) = types[outer] {
            let mut inner = outer + 1;
            while let Some((inner_type, inner_ver)) = types[inner] {
                if str_eq(inner_type, outer_type) && inner_ver == outer_ver {
                    return false;
                }
                inner += 1;
            }
            outer += 1;
        }

        true
    }
}

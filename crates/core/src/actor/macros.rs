#[macro_export]
macro_rules! messages {
    ( $($name:ident $( { $($f_name:ident : $f_typ:ty),* $(,)? } )? $( ( $($t_typ:ty),* $(,)? ) )? ),* $(,)? ) => {
        $(
            $crate::messages!(@dispatch $name $( { $($f_name : $f_typ),* } )? $( ( $($t_typ),* ) )? );
        )*
    };

    (@dispatch $name:ident { $($f_name:ident : $f_typ:ty),* $(,)? } ) => {
        #[derive(Debug, Clone)]
        pub struct $name {
            $( pub $f_name : $f_typ ),*
        }
        impl Message for $name {}
    };

    (@dispatch $name:ident ( $first:ty $(, $rest:ty)* $(,)? ) ) => {
        #[derive(Debug, Clone)]
        pub struct $name(pub $first, $(pub $rest),*);
        impl Message for $name {}
    };

    (@dispatch $name:ident $($_:tt)? ) => {
        #[derive(Debug, Clone)]
        pub struct $name;
        impl Message for $name {}
    };
}

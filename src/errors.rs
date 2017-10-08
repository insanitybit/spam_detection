use std::borrow::Cow;

error_chain! {
    errors {
        RecoverableError(t: Cow<'static, str>) {
            description("A retryable operation failed.")
            display("{}", t)
        }
        UnrecoverableError(t: Cow<'static, str>) {
            description("An unrecoverable error occurred.")
            display("{}", t)
        }
    }


}
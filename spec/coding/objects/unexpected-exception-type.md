# UnexpectedExceptionType

`UnexpectedExceptionType` owns all exception causes not claimed by a concrete binding. It records `scause`, `sepc`,
`stval`, CPU/task/Flow identities and stack bounds, then fail-stops. It never silently returns or opens nesting.

Mapping: charter [`unexpected-exception-type.md`](../../charter/objects/unexpected-exception-type.md), model
[`unexpected_exception_type.spec`](../../model/objects/unexpected_exception_type.spec), implementation
[`unexpected_exception_type.rs`](../../../impl/arceos_ex/src/objects/unexpected_exception_type.rs).

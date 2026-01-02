%: %.rs
	rustc -C opt-level=z -C lto=thin -C codegen-units=1 -C panic=abort $^

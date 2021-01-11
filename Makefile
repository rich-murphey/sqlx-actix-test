
run:
	cargo run --release

test:
	curl -H "Content-Type: application/json" -d '{"offset":0,"limit":1}' http://127.0.0.1:8080/junk
	@echo
	curl -H "Content-Type: application/json" -d '{"offset":0,"limit":1}' http://127.0.0.1:8080/junkstream
	@echo

bench:
	drill --stats -q --benchmark tests/local.yml

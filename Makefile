
run:
	cargo run --release

test:
	curl -H "Content-Type: application/json" -d '{"offset":0,"limit":1}' http://127.0.0.1:8080/junk
	@echo
	curl -H "Content-Type: application/json" -d '{"offset":0,"limit":1}' http://127.0.0.1:8080/junkstream
	@echo

bench:
	drill --stats -q --benchmark tests/local.yml

hey:
	hey -n 10000 -c 64 -m POST -H "Content-Type: application/json" -d '{"offset":0,"limit":256}' http://127.0.0.1:8080/junk
	hey -n 10000 -c 64 -m POST -H "Content-Type: application/json" -d '{"offset":0,"limit":256}' http://127.0.0.1:8080/junkstream

# warn up using drill, then switch to wrk to validate each response length
wrk:
	drill -q --benchmark tests/warmup.yml
	wrk -c200 -t24 -d20s -s tests/junkstream.lua http://127.0.0.1:8080

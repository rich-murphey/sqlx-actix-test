
run:
	cargo run --release

# this uses curl to shows a single item for each REST method.
test:
	curl -H "Content-Type: application/json" -d '{"offset":0,"limit":1}' http://127.0.0.1:8080/junk
	@echo
	curl -H "Content-Type: application/json" -d '{"offset":0,"limit":1}' http://127.0.0.1:8080/junkstream
	@echo

# drill measures throuput and average latency
bench:
	drill --stats -q --benchmark tests/local.yml

# hey measures tail latency
hey:
	hey -n 10000 -c 64 -m POST -H "Content-Type: application/json" -d '{"offset":0,"limit":256}' http://127.0.0.1:8080/junk
	hey -n 10000 -c 64 -m POST -H "Content-Type: application/json" -d '{"offset":0,"limit":256}' http://127.0.0.1:8080/junkstream

# wrk validates the response payload length, in order to detect any invalid responses.
wrk:
	wrk -c200 -t24 -d8s -s tests/junkstream.lua http://127.0.0.1:8080
	wrk -c200 -t24 -d8s -s tests/junk.lua http://127.0.0.1:8080

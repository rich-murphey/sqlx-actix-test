
run:
	cargo run --release

# drill measures throuput and average latency
# this reproduces the protocol error.
test:
	drill --stats -q --benchmark tests/local.yml

# check the output of each REST method.
check:
	ht -j http://127.0.0.1:8080/junk limit:=1 offset:=0
	ht -j http://127.0.0.1:8080/junkstream/1/0

# hey measures tail latency
hey:
	hey -n 10000 -c 64 -m POST -H "Content-Type: application/json" -d '{"offset":0,"limit":256}' http://127.0.0.1:8080/junk
	hey -n 10000 -c 64 -m POST -H "Content-Type: application/json" -d '{"offset":0,"limit":256}' http://127.0.0.1:8080/junkstream

# wrk validates the response payload length, in order to detect any invalid responses.
wrk:
	wrk -c200 -t24 -d8s -s tests/junkstream.lua http://127.0.0.1:8080
	wrk -c200 -t24 -d8s -s tests/junk.lua http://127.0.0.1:8080

outdated:
	-cargo +nightly udeps --release 2>&1 |grep -v 'Loading save analysis'
	-cargo tree -d
	-cargo outdated -R 2>&1 |grep -v rt-threaded

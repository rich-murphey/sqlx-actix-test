--- -*- compile-command: "wrk -c200 -t24 -d20s -s junkstream.lua http://127.0.0.1:8080"; -*-
wrk.method  = "POST"
wrk.path    = "/junkstream"
wrk.body    = "{\"offset\":0,\"limit\":100}"
wrk.headers["Content-Type"] = "application/json;charset=utf-8"

function response(status, headers, body)
   len = string.len(body)
   if len ~= 2684983 then
      print(len)
   end
end

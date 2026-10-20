import http from "node:http";
http.createServer((_, res) => res.end("api\n")).listen(3001, () => console.log("api on :3001"));

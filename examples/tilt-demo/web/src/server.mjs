import http from "node:http";
http.createServer((_, res) => res.end("web\n")).listen(3000, () => console.log("web on :3000"));

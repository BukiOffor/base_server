// Local-only stand-in for the deployment's SMS/WhatsApp/email adapter.
const http = require('http');
let messages = [];
http.createServer(async (req, res) => {
  const path = req.url.split('?')[0];
  res.setHeader('Content-Type', 'application/json');
  if (req.method === 'POST' && path === '/messages') {
    let body = req.body;
    if (!body) {
      let raw = ''; for await (const chunk of req) raw += chunk;
      try { body = JSON.parse(raw); } catch { res.writeHead(400); res.end('{}'); return; }
    }
    if (!['team-code', 'password-reset'].includes(body.kind) || !body.recipient || !body.value) {
      res.writeHead(400); res.end('{}'); return;
    }
    messages.push(body);
    res.writeHead(204); res.end();
  } else if (req.method === 'GET' && path === '/messages') {
    res.end(JSON.stringify(messages));
  } else if (req.method === 'DELETE' && path === '/messages') {
    messages = []; res.writeHead(204); res.end();
  } else { res.writeHead(501); res.end('{}'); }
}).listen(process.env.PORT);

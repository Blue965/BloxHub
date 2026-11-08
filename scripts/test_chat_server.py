#!/usr/bin/env python3
"""Test protocolaire du serveur BloxHub Chat (2 clients + scénario complet)."""
import asyncio, json, subprocess, sys, time
import websockets

URL = "ws://127.0.0.1:8799"
results = []

def check(name, ok, detail=""):
    results.append((name, ok))
    print(("✅" if ok else "❌") + f" {name}" + (f" — {detail}" if detail and not ok else ""))

async def recv_until(ws, mtype, timeout=4):
    """Lit des messages jusqu'au type voulu (les autres sont retournés dans une liste)."""
    seen = []
    end = time.time() + timeout
    while time.time() < end:
        try:
            raw = await asyncio.wait_for(ws.recv(), timeout=max(0.1, end - time.time()))
        except asyncio.TimeoutError:
            break
        msg = json.loads(raw)
        if msg.get("type") == mtype:
            return msg, seen
        seen.append(msg)
    raise TimeoutError(f"type {mtype} non reçu (vu: {[m.get('type') for m in seen]})")

async def main():
    async with websockets.connect(URL) as a, websockets.connect(URL) as b:
        # hello
        await a.send(json.dumps({"type": "hello", "username": "Alice"}))
        w, _ = await recv_until(a, "welcome")
        check("welcome Alice", w.get("username") is None and w.get("user_id") is not None and "general" in w.get("joined", []))
        check("salons par défaut", set(w.get("rooms", [])) >= {"general", "fastflags", "jeux", "aide", "off-topic"})

        await b.send(json.dumps({"type": "hello", "username": "bob"}))
        wb, _ = await recv_until(b, "welcome")
        check("welcome Bob", wb.get("user_id") is not None)

        # présence (on attend l'événement qui contient les DEUX users)
        names = set()
        end = time.time() + 5
        while time.time() < end and names != {"Alice", "bob"}:
            try:
                p = json.loads(await asyncio.wait_for(a.recv(), timeout=1))
            except asyncio.TimeoutError:
                continue
            if p.get("type") == "presence":
                names = {u["username"] for u in p.get("users", [])}
        check("présence 2 users", names == {"Alice", "bob"}, str(names))

        # join + message
        await a.send(json.dumps({"type": "join", "room": "fastflags"}))
        await recv_until(a, "room_history")
        await b.send(json.dumps({"type": "join", "room": "fastflags"}))
        await recv_until(b, "room_history")
        await b.send(json.dumps({"type": "message", "room": "fastflags", "content": "DFIntTaskSchedulerTargetFps = 9999 !"}))
        m, _ = await recv_until(a, "message")
        check("broadcast salon", m["from"] == "bob" and "9999" in m["content"] and m["room"] == "fastflags" and "color" in m)

        # dm
        await a.send(json.dumps({"type": "dm", "to": "bob", "content": "salut secret"}))
        dmb, _ = await recv_until(b, "dm")
        check("DM reçu par Bob", dmb["from"] == "Alice" and dmb["content"] == "salut secret" and dmb["from_self"] is False)
        dma, _ = await recv_until(a, "dm")
        check("écho DM à l'expéditeur", dma["from_self"] is True and dma.get("to") == "bob")

        # typing
        await a.send(json.dumps({"type": "typing", "room": "fastflags"}))
        tp, _ = await recv_until(b, "typing")
        check("indicateur typing", tp["from"] == "Alice" and tp["room"] == "fastflags")

        # pseudo dupliqué (casse ignorée)
        async with websockets.connect(URL) as c:
            await c.send(json.dumps({"type": "hello", "username": "ALICE"}))
            err, _ = await recv_until(c, "error")
            check("rejet pseudo dupliqué", err.get("code") == "username_taken")

        # flood : 12 messages d'affilée → au moins une erreur rate_limited
        for i in range(12):
            await b.send(json.dumps({"type": "message", "room": "general", "content": f"spam {i}"}))
        got_limit = False
        try:
            end = time.time() + 3
            while time.time() < end:
                msg = json.loads(await asyncio.wait_for(b.recv(), timeout=1))
                if msg.get("type") == "error" and msg.get("code") == "rate_limited":
                    got_limit = True
                    break
        except asyncio.TimeoutError:
            pass
        check("anti-flood actif", got_limit)

    # persistance : l'historique survit au redémarrage
    subprocess.run(["pkill", "-f", "bloxhub-server"], check=False)
    time.sleep(0.6)
    subprocess.Popen(["/home/z/my-project/bloxhub/target/debug/bloxhub-server", "--port", "8799", "--data", "/tmp/chat-test"],
                     stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    time.sleep(1.0)
    async with websockets.connect(URL) as d:
        await d.send(json.dumps({"type": "hello", "username": "Zoe"}))
        w2, _ = await recv_until(d, "welcome")
        hist = w2.get("history", {}).get("general", [])
        check("historique persistant", len(hist) >= 5 and any("spam" in m["content"] for m in hist))

    failed = [n for n, ok in results if not ok]
    print(f"\n{len(results) - len(failed)}/{len(results)} tests réussis")
    sys.exit(1 if failed else 0)

asyncio.run(main())

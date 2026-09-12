import { spawn } from "node:child_process";
import { createConnection } from "node:net";
import { existsSync, unlinkSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";

export function mpvLaunchOptions(platform, pipePath) {
  const common=["--idle=yes","--force-window=immediate","--keep-open=no","--input-ipc-server="+pipePath,"--video-sync=audio"];
  return platform==="win32"?[...common,"--vo=gpu","--gpu-api=opengl","--gpu-context=win","--hwdec=d3d11va-copy","--opengl-swapinterval=1","--priority=high"]:[...common,"--vo=gpu-next","--hwdec=auto-safe"];
}

export function sortMediaFiles(files) {
  const compare = new Intl.Collator("ru", { numeric: true, sensitivity: "base" }).compare;
  return [...files].sort((a, b) => {
    const groupA = String(a.path).split(/[\\/]/).slice(0, -1).join("/");
    const groupB = String(b.path).split(/[\\/]/).slice(0, -1).join("/");
    // Separate collections must not be interleaved by equal episode numbers.
    if (a.season != null && b.season != null && groupA.replace(/(?:season|сезон|s)[ ._-]*\d+/ig, "") === groupB.replace(/(?:season|сезон|s)[ ._-]*\d+/ig, "")) {
      return a.season - b.season || (a.episode ?? 0) - (b.episode ?? 0) || compare(a.path, b.path) || a.id - b.id;
    }
    return compare(groupA, groupB) || (a.episode ?? 0) - (b.episode ?? 0) || compare(a.name, b.name) || a.id - b.id;
  });
}

export function resumePosition(history, mode = "resume") {
  if (mode === "start") return 0;
  const time = Math.max(0, Number(history?.playbackTimecode) || 0);
  return time;
}

export class PlaybackError extends Error {
  constructor(message, status = 400, active = []) { super(message); this.status = status; this.active = active; }
}

export class MpvPlayer {
  constructor({ db, client, executable, spawnProcess = spawn, connect = createConnection, focusWindow, platform = process.platform }) {
    this.db = db; this.client = client; this.executable = executable;
    this.spawnProcess = spawnProcess; this.connect = connect; this.platform = platform;
    this.focusWindow = focusWindow ?? ((pid) => {
      if (process.platform !== "win32" || !Number.isInteger(pid)) return;
      const child = spawn("powershell.exe", ["-NoProfile", "-NonInteractive", "-WindowStyle", "Hidden", "-Command",
        "(New-Object -ComObject WScript.Shell).AppActivate(" + pid + ") | Out-Null"], { windowsHide: true, stdio: "ignore" });
      child.on("error", () => { /* IPC restore still works if Windows denies foreground focus. */ });
    });
    // ponytail: serialize local player commands; use per-window locks if concurrent control becomes necessary.
    this.sessions = new Map(); this.counter = 0; this.serial = Promise.resolve();
  }

  exclusive(fn) {
    const result = this.serial.then(fn);
    this.serial = result.catch(() => {});
    return result;
  }

  list() {
    return [...this.sessions.values()].filter(s => !s.closed).map(s => ({
      id: s.id, hash: s.hash, fileIndex: s.file?.id, fileName: s.file?.name ?? "",
      timecode: s.timecode, duration: s.duration, loading: s.loading, ended: s.ended,
      autoNext: s.autoNext, error: s.error,
      canNext: Boolean(s.file) && s.files.findIndex(f => f.id === s.file.id) < s.files.length - 1,
      nextName: s.files[s.files.findIndex(f => f.id === s.file?.id) + 1]?.name ?? null,
    }));
  }

  persist(s, force = false) {
    if (!s.marked || !s.file || (!force && Date.now() - s.lastSaved < 5000)) return;
    this.db.updateFileProgress(s.hash, s.file.id, { timecode: s.timecode, duration: s.duration });
    s.lastSaved = Date.now();
  }

  command(s, command) {
    if (s.closed || !s.socket || s.socket.destroyed) return Promise.reject(new PlaybackError("Связь с MPV потеряна", 503));
    const requestId = ++s.requestId;
    return new Promise((resolve, reject) => {
      const timer = setTimeout(() => { s.pending.delete(requestId); reject(new PlaybackError("MPV не ответил на команду", 504)); }, 5000);
      s.pending.set(requestId, { resolve, reject, timer });
      s.socket.write(JSON.stringify({ command, request_id: requestId }) + "\n", error => {
        if (error) { clearTimeout(timer); s.pending.delete(requestId); reject(error); }
      });
    });
  }

  handle(s, event) {
    const pending = s.pending.get(event.request_id);
    if (pending) {
      clearTimeout(pending.timer); s.pending.delete(event.request_id);
      if (event.error && event.error !== "success") pending.reject(new PlaybackError("MPV: " + event.error));
      else pending.resolve(event.data);
      return;
    }
    if (s.closed) return;
    if (event.event === "start-file") s.receiving = true;
    if (event.event === "file-loaded" && s.receiving && !s.marked) {
      s.loading = false; s.marked = true;
      this.db.markFilePlayed(s.hash, { fileIndex: s.file.id, fileName: s.file.name, filePath: s.file.path });
      this.persist(s, true);
    }
    if (event.event === "property-change" && s.receiving) {
      if (event.name === "time-pos" && Number.isFinite(event.data)) s.timecode = Math.max(0, event.data);
      if (event.name === "duration" && Number.isFinite(event.data)) s.duration = Math.max(0, event.data);
      if (event.name === "aid" && Number.isSafeInteger(event.data) && s.hash) this.db.saveAudioTrack(s.hash, event.data);
      this.persist(s);
    }
    if (event.event === "end-file" && s.receiving) {
      this.persist(s, true); s.receiving = false; s.loading = false; s.ended = true;
      if (event.reason === "error") s.error = "MPV не смог прочитать поток. Проверьте раздачу и повторите запуск.";
      if (event.reason === "eof" && s.autoNext) {
        void this.exclusive(async () => {
          if (!s.closed && s.ended && s.autoNext) await this.next(s);
        }).catch(error => { s.error = error.message; });
      }
    }
  }

  async attach(s) {
    for (let attempt = 0; attempt < 40; attempt++) {
      if (s.closed) throw new PlaybackError("MPV завершился до подключения IPC", 503);
      try {
        await new Promise((resolve, reject) => {
          const socket = this.connect(s.pipePath);
          let buffer = ""; let connected = false;
          socket.setEncoding("utf8");
          socket.once("connect", () => { connected = true; s.socket = socket; resolve(); });
          socket.on("data", chunk => {
            buffer += chunk;
            if (buffer.length > 1024 * 1024) { socket.destroy(); return; }
            const lines = buffer.split("\n"); buffer = lines.pop() ?? "";
            for (const line of lines) {
              let message;
              try { message = JSON.parse(line); } catch { continue; }
              try { this.handle(s, message); } catch (error) { s.error = error.message; }
            }
          });
          socket.on("error", error => { if (!connected) reject(error); else s.error = "Ошибка соединения с MPV"; });
          socket.on("close", () => { if (connected && !s.closed) this.dispose(s); });
        });
        await this.command(s, ["observe_property", 1, "time-pos"]);
        await this.command(s, ["observe_property", 2, "duration"]);
        await this.command(s, ["observe_property", 3, "aid"]);
        return;
      } catch (error) {
        if (s.socket || attempt === 39) throw error;
        await new Promise(resolve => setTimeout(resolve, 100));
      }
    }
  }

  async create() {
    if (!this.executable) throw new PlaybackError("Встроенный MPV не найден", 503);
    const id = String(process.pid) + "-" + (++this.counter);
    const pipePath=this.platform==="win32"?"\\\\.\\pipe\\pirate-cinema-mpv-"+id:join(tmpdir(),"pirate-cinema-mpv-"+id+".sock");
    const s = { id, pipePath, socket: null, child: null,
      requestId: 0, pending: new Map(), closed: false, marked: false, receiving: false,
      file: null, files: [], timecode: 0, duration: 0, lastSaved: 0, error: null };
    this.sessions.set(id, s);
    try {
      const child = this.spawnProcess(this.executable, mpvLaunchOptions(this.platform,s.pipePath),
      { windowsHide: false, stdio: "ignore" });
      s.child = child;
      child.once("exit", () => this.dispose(s, false));
      child.once("error", () => this.dispose(s, false));
      await this.attach(s);
      return s;
    } catch (error) { this.dispose(s); throw error; }
  }

  async load(s, hash, files, file, mode, autoNext) {
    if ([...this.sessions.values()].some(other => other !== s && !other.closed && !other.ended && other.hash === hash && other.file?.id === file.id))
      throw new PlaybackError("Следующий файл уже открыт в другом окне MPV. Закройте то окно перед переходом.", 409);
    this.persist(s, true);
    const history = this.db.listFileHistory(hash).find(h => Number(h.fileIndex) === file.id);
    const resume = resumePosition(history, mode);
    Object.assign(s, { hash, files, file, timecode: resume, duration: Number(history?.playbackDuration) || 0,
      marked: false, receiving: false, loading: true, ended: false, error: null, autoNext });
    try {
      const audioTrack=this.db.getAudioTrack(hash);
      const options={ start: String(resume), "force-media-title": file.name, ...(audioTrack?{aid:String(audioTrack)}:{}) };
      await this.command(s, ["loadfile", this.client.streamUrl(hash, file.id, file.name), "replace", -1, options]);
    } catch (error) { s.loading = false; s.ended = true; s.error = error.message; throw error; }
    return { ...this.list().find(item => item.id === s.id), resumeSeconds: resume };
  }

  play({ hash, files, fileIndex, mode = "resume", existing = "ask", sessionId, autoNext = false }) {
    return this.exclusive(async () => {
      if (!["resume", "start"].includes(mode) || !["ask", "replace", "new"].includes(existing) || typeof autoNext !== "boolean")
        throw new PlaybackError("Некорректные параметры воспроизведения");
      files = sortMediaFiles(files);
      const file = files.find(f => f.id === fileIndex);
      if (!file) throw new PlaybackError("Файл отсутствует в раздаче", 404);
      const same = [...this.sessions.values()].find(s => !s.closed && !s.ended && s.hash === hash && s.file?.id === fileIndex);
      if (same) { await this.focus(same); return { ...this.list().find(s => s.id === same.id), focused: true }; }
      const active = this.list();
      if (active.length && existing === "ask") throw new PlaybackError("MPV уже открыт", 409, active);
      let s;
      if (existing === "replace" && active.length) {
        s = this.sessions.get(sessionId);
        if (!s) throw new PlaybackError("Выберите окно MPV для замены", 409, active);
      } else s = await this.create();
      return this.load(s, hash, files, file, mode, autoNext);
    });
  }

  async focus(s) {
    await this.command(s, ["set_property", "window-minimized", false]);
    this.focusWindow(s.child.pid);
  }

  async next(s) {
    const index = s.files.findIndex(f => f.id === s.file?.id);
    const file = s.files[index + 1];
    if (!file) return;
    await this.load(s, s.hash, s.files, file, "resume", s.autoNext);
  }

  control(id, action, value) {
    return this.exclusive(async () => {
      const s = this.sessions.get(id);
      if (!s || s.closed) throw new PlaybackError("Окно MPV уже закрыто", 404);
      if (action === "focus") await this.focus(s);
      else if (action === "next") {
        if (s.loading) throw new PlaybackError("Дождитесь открытия текущего файла", 409);
        await this.next(s);
      }
      else if (action === "autoNext" && typeof value === "boolean") s.autoNext = value;
      else if (action === "stop") this.dispose(s);
      else throw new PlaybackError("Неизвестная команда MPV");
      return this.list();
    });
  }

  isOpen(hash, index) { return [...this.sessions.values()].some(s => !s.closed && s.hash === hash && s.file?.id === index); }

  dispose(s, kill = true) {
    if (s.closed) return;
    try { this.persist(s, true); } catch (error) {
      console.error("Failed to save MPV progress:", error);
    } finally {
      s.closed = true;
      for (const p of s.pending.values()) { clearTimeout(p.timer); p.reject(new PlaybackError("MPV закрыт", 503)); }
      s.pending.clear(); s.socket?.destroy();
      if (kill && s.child && !s.child.killed) s.child.kill();
      if(this.platform!=="win32"&&existsSync(s.pipePath))try{unlinkSync(s.pipePath)}catch{/* MPV may remove its own socket first. */}
      this.sessions.delete(s.id);
    }
  }

  close() { for (const s of this.sessions.values()) this.dispose(s); }
}

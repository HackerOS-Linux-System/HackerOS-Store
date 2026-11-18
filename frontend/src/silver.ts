type Listener = (e: { payload: any }) => void;
const listeners: Record<string, Set<Listener>> = {};

interface Job { id: string; offset: number; resolve: (v: any) => void; reject: (e: any) => void }
const jobs = new Map<string, Job>();
let timer: any = null;
let polling = false;
let quietPolls = 0; // kolejne odpytania bez nowych zdarzeń — im dłużej cisza, tym rzadziej pytamy

function dispatch(name: string, payload: any) {
  const set = listeners[name];
  if (!set) return;
  for (const cb of Array.from(set)) {
    try { cb({ payload }); } catch (e) { console.error(e); }
  }
}

async function pollOnce() {
  if (polling) return;
  polling = true;
  try {
    for (const job of Array.from(jobs.values())) {
      let r: any;
      try {
        r = await silver.invoke("job_poll", { id: job.id, offset: job.offset });
      } catch (e) {
        jobs.delete(job.id);
        job.reject(String(e));
        continue;
      }
      job.offset = r.offset;
      if (r.chunk) quietPolls = 0; else quietPolls++;
      if (r.chunk) {
        for (const line of String(r.chunk).split("\n")) {
          if (!line) continue;
          try {
            const ev = JSON.parse(line);
            dispatch(ev.e, ev.p);
          } catch { /* niepełna linia */ }
        }
      }
      if (r.done) {
        jobs.delete(job.id);
        if (r.ok) job.resolve(r.value === undefined ? null : r.value);
        else job.reject(r.error ?? "unknown error");
      }
    }
  } finally {
    polling = false;
    if (jobs.size === 0) { if (timer) { clearTimeout(timer); timer = null; } quietPolls = 0; }
    else schedule();
  }
}

// Każde odpytanie to wywołanie do H# (alokacje bez GC) — przy braku zdarzeń odpytujemy rzadziej.
function schedule() {
  if (timer) clearTimeout(timer);
  const delay = quietPolls < 10 ? 150 : quietPolls < 40 ? 300 : 500;
  timer = setTimeout(() => { timer = null; void pollOnce(); }, delay);
}

export async function invoke<T = any>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  if (cmd === "cancel_install") {
    await silver.invoke("cancel_install", {});
    return undefined as T;
  }
  const started = await silver.invoke("job_start", { cmd, args: JSON.stringify(args ?? {}) });
  return new Promise<T>((resolve, reject) => {
    jobs.set(started.id, { id: started.id, offset: 0, resolve, reject });
    quietPolls = 0;
    if (!timer && !polling) schedule();
  });
}

export async function listen<T = any>(event: string, cb: (e: { payload: T }) => void): Promise<() => void> {
  (listeners[event] ??= new Set()).add(cb as Listener);
  return () => { listeners[event]?.delete(cb as Listener); };
}

// Strażnik pamięci: co 5 s puls z informacją, od ilu sekund użytkownik nic nie robi.
// Backend może odświeżyć okno WYŁĄCZNIE gdy jest bezczynność i bardzo wysokie zużycie RAM (patrz main.h#).
let lastInput = Date.now();
try {
  const touch = () => { lastInput = Date.now(); };
  for (const ev of ["mousedown", "keydown", "wheel"]) {
    (document as any).addEventListener(ev, touch);
  }
} catch (_) {}
try {
  setInterval(() => {
    try {
      const idle = Math.floor((Date.now() - lastInput) / 1000);
      if (jobs.size > 0) return; // trwa operacja — nie ruszamy okna
      (silver as any).invoke("job_poll", { id: "", offset: 0, idle });
    } catch (_) {}
  }, 5000);
} catch (_) {}

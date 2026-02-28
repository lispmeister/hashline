import type { EventRecord } from "./types";

interface EventEnvelope {
  type?: string;
  timestamp?: number;
  sessionID?: string;
  properties?: Record<string, unknown>;
  error?: { message?: string; name?: string };
}

interface EventStream {
  stream: AsyncIterable<EventEnvelope>;
}

interface OpenCodeClientLike {
  event: {
    subscribe: () => Promise<EventStream>;
  };
}

export interface EventCapture {
  stop: () => void;
  eventsForSession: (sessionID: string) => EventRecord[];
}

function classifyRetry(ev: EventEnvelope): boolean {
  const t = ev.type ?? "";
  if (t.includes("retry")) return true;
  const msg = ev.error?.message ?? "";
  return msg.toLowerCase().includes("retry");
}

function classifyError(ev: EventEnvelope): boolean {
  const t = ev.type ?? "";
  if (t.includes("error")) return true;
  return Boolean(ev.error);
}

export async function startEventCapture(client: OpenCodeClientLike): Promise<EventCapture> {
  const allEvents: EventRecord[] = [];
  let running = true;

  const sub = await client.event.subscribe();

  void (async () => {
    for await (const ev of sub.stream) {
      if (!running) break;
      const flags: string[] = [];
      if (classifyRetry(ev)) flags.push("retry");
      if (classifyError(ev)) flags.push("error");

      allEvents.push({
        timestamp: ev.timestamp ?? Date.now(),
        type: flags.length > 0 ? `${ev.type ?? "event"}:${flags.join(",")}` : ev.type ?? "event",
        sessionID: ev.sessionID,
        message: ev.error?.message,
        raw: ev,
      });
    }
  })();

  return {
    stop: () => {
      running = false;
    },
    eventsForSession: (sessionID: string) => allEvents.filter((e) => e.sessionID === sessionID),
  };
}

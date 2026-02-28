import type { ModelRef } from "./types";

interface SessionMessageInfo {
  role?: string;
  providerID?: string;
  modelID?: string;
}

interface SessionMessage {
  info?: SessionMessageInfo;
}

interface OpenCodeClientLike {
  project: {
    current: () => Promise<{ data: { id: string } }>;
  };
  session: {
    list: () => Promise<{ data: Array<{ id: string; projectID?: string; projectId?: string }> }>;
    messages: (args: { path: { id: string } }) => Promise<{ data: SessionMessage[] }>;
  };
}

export function parseModelString(value: string): ModelRef {
  const [providerID, modelID] = value.split("/");
  if (!providerID || !modelID) {
    throw new Error(`Invalid model '${value}'. Use provider/model format.`);
  }
  return { providerID, modelID, label: `${providerID}/${modelID}` };
}

export async function resolveModel(client: OpenCodeClientLike, explicitModel?: string): Promise<ModelRef> {
  if (explicitModel) {
    return parseModelString(explicitModel);
  }

  const currentProject = await client.project.current();
  const sessions = await client.session.list();

  const candidateSessions = sessions.data.filter((s) => {
    const pid = s.projectID ?? s.projectId;
    return pid === currentProject.data.id;
  });

  for (const session of candidateSessions) {
    const messages = await client.session.messages({ path: { id: session.id } });
    const reversed = [...messages.data].reverse();
    const assistant = reversed.find((m) => m.info?.role === "assistant");

    if (assistant?.info?.providerID && assistant.info.modelID) {
      const providerID = assistant.info.providerID;
      const modelID = assistant.info.modelID;
      return { providerID, modelID, label: `${providerID}/${modelID}` };
    }
  }

  throw new Error("Unable to infer active model. Pass --model provider/model.");
}

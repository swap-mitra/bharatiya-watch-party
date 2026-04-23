import type {
  ClientEnvelope,
  CreateRoomResponse,
  JoinRoomResponse,
  RoomSession,
} from './types';

const SIGNAL_HTTP_BASE =
  (import.meta.env.VITE_SIGNAL_SERVICE_URL as string | undefined) ?? 'http://127.0.0.1:4000';
const SIGNAL_WS_BASE =
  (import.meta.env.VITE_SIGNAL_SERVICE_WS_URL as string | undefined) ?? 'ws://127.0.0.1:4000';

async function postJson<TResponse>(path: string, body: object): Promise<TResponse> {
  const response = await fetch(`${SIGNAL_HTTP_BASE}${path}`, {
    method: 'POST',
    headers: {
      'Content-Type': 'application/json',
    },
    body: JSON.stringify(body),
  });

  if (!response.ok) {
    let message = `Request failed with ${response.status}`;
    try {
      const payload = (await response.json()) as { error?: string };
      if (payload.error) {
        message = payload.error;
      }
    } catch {
      // Ignore JSON parsing failures and keep the HTTP status message.
    }
    throw new Error(message);
  }

  return (await response.json()) as TResponse;
}

export function createRoom(displayName: string): Promise<CreateRoomResponse> {
  return postJson<CreateRoomResponse>('/api/rooms', {
    displayName,
  });
}

export function joinRoom(roomCode: string, displayName: string): Promise<JoinRoomResponse> {
  return postJson<JoinRoomResponse>(`/api/rooms/${encodeURIComponent(roomCode)}/join`, {
    displayName,
  });
}

export function connectRoomSocket(session: RoomSession): WebSocket {
  const url = new URL('/ws', SIGNAL_WS_BASE);
  url.searchParams.set('room_code', session.roomCode);
  url.searchParams.set('session_id', session.sessionId);
  return new WebSocket(url);
}

export function sendEnvelope(socket: WebSocket | null, envelope: ClientEnvelope): void {
  if (!socket || socket.readyState !== WebSocket.OPEN) {
    throw new Error('Room connection is not open');
  }

  socket.send(JSON.stringify(envelope));
}

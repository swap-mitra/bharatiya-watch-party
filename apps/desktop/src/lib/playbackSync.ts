import type { PlaybackHeartbeat, PlayerState, RoomSession } from './types';

export const HOST_HEARTBEAT_INTERVAL_MS = 2000;

const PAUSED_DRIFT_SEEK_MS = 500;
const PLAYING_DRIFT_SEEK_MS = 750;
const PLAYING_DRIFT_HARD_SEEK_MS = 3000;
const MIN_CORRECTION_INTERVAL_MS = 1000;

const heartbeatStatuses = new Set<PlayerState['status']>(['playing', 'paused', 'buffering', 'loading']);

export interface BuildHeartbeatInput {
  session: RoomSession | null;
  state: PlayerState;
  commandSeq: number;
  nowMs: number;
}

export interface ResolveHeartbeatInput {
  current: PlayerState;
  heartbeat: PlaybackHeartbeat;
  isHost: boolean;
  lastCommandSeq: number;
  lastCorrectionAtMs: number;
  nowMs: number;
}

export type HeartbeatPlan =
  | { kind: 'ignore' }
  | {
      kind: 'apply';
      loadSource?: string;
      seekToMs?: number;
      playbackIntent?: PlaybackIntent;
      logMessage?: string;
      correctionAtMs?: number;
    };

type PlaybackIntent = 'play' | 'pause';

export function buildHostHeartbeat(input: BuildHeartbeatInput): PlaybackHeartbeat | null {
  if (input.session?.role !== 'host' || !input.state.activeSource) {
    return null;
  }

  if (!heartbeatStatuses.has(input.state.status)) {
    return null;
  }

  return {
    commandSeq: input.commandSeq,
    positionMs: input.state.positionMs,
    status: input.state.status,
    activeSource: input.state.activeSource,
    sentAtMs: input.nowMs,
  };
}

export function resolveHeartbeatPlan(input: ResolveHeartbeatInput): HeartbeatPlan {
  const { current, heartbeat, isHost, lastCommandSeq, lastCorrectionAtMs, nowMs } = input;

  if (isHost || heartbeat.commandSeq < lastCommandSeq || !heartbeat.activeSource) {
    return { kind: 'ignore' };
  }

  if (current.activeSource !== heartbeat.activeSource) {
    return {
      kind: 'apply',
      loadSource: heartbeat.activeSource,
      seekToMs: heartbeat.positionMs,
      playbackIntent: playbackIntentForStatus(heartbeat.status),
      logMessage: 'Synced to host source heartbeat',
    };
  }

  const driftMs = current.positionMs - heartbeat.positionMs;
  const absoluteDriftMs = Math.abs(driftMs);

  if (heartbeat.status === 'paused') {
    return {
      kind: 'apply',
      seekToMs: absoluteDriftMs > PAUSED_DRIFT_SEEK_MS ? heartbeat.positionMs : undefined,
      playbackIntent: current.status === 'paused' ? undefined : 'pause',
    };
  }

  if (heartbeat.status !== 'playing') {
    return { kind: 'ignore' };
  }

  const shouldCorrectDrift =
    absoluteDriftMs >= PLAYING_DRIFT_SEEK_MS && nowMs - lastCorrectionAtMs >= MIN_CORRECTION_INTERVAL_MS;

  return {
    kind: 'apply',
    playbackIntent: current.status === 'playing' ? undefined : 'play',
    seekToMs: shouldCorrectDrift ? heartbeat.positionMs : undefined,
    correctionAtMs: shouldCorrectDrift ? nowMs : undefined,
    logMessage: shouldCorrectDrift
      ? absoluteDriftMs >= PLAYING_DRIFT_HARD_SEEK_MS
        ? `Hard synced ${absoluteDriftMs}ms drift`
        : `Corrected ${absoluteDriftMs}ms drift`
      : undefined,
  };
}

function playbackIntentForStatus(status: PlayerState['status']): PlaybackIntent | undefined {
  if (status === 'playing') {
    return 'play';
  }
  if (status === 'paused') {
    return 'pause';
  }
  return undefined;
}

import type { PlaybackHeartbeat, PlayerState, RoomSession } from './types';

export const HOST_HEARTBEAT_INTERVAL_MS = 2000;

const PAUSED_DRIFT_SEEK_MS = 500;
const PLAYING_DRIFT_NOOP_MS = 250;
const PLAYING_DRIFT_RATE_MS = 2500;
const PLAYING_DRIFT_HARD_SEEK_MS = 5000;
const MIN_CORRECTION_INTERVAL_MS = 1000;
const NORMAL_RATE_PERCENT = 100;
const SLOW_RATE_PERCENT = 97;
const FAST_RATE_PERCENT = 103;

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
      playbackRatePercent?: number;
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
      playbackRatePercent: NORMAL_RATE_PERCENT,
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
      playbackRatePercent: NORMAL_RATE_PERCENT,
    };
  }

  if (heartbeat.status !== 'playing') {
    return { kind: 'ignore' };
  }

  if (absoluteDriftMs < PLAYING_DRIFT_NOOP_MS) {
    return {
      kind: 'apply',
      playbackIntent: current.status === 'playing' ? undefined : 'play',
      playbackRatePercent:
        current.playbackRatePercent === NORMAL_RATE_PERCENT ? undefined : NORMAL_RATE_PERCENT,
    };
  }

  if (absoluteDriftMs < PLAYING_DRIFT_RATE_MS) {
    const playbackRatePercent = driftMs > 0 ? SLOW_RATE_PERCENT : FAST_RATE_PERCENT;
    return {
      kind: 'apply',
      playbackIntent: current.status === 'playing' ? undefined : 'play',
      playbackRatePercent:
        current.playbackRatePercent === playbackRatePercent ? undefined : playbackRatePercent,
      logMessage: `Smoothing ${absoluteDriftMs}ms drift at ${playbackRatePercent}% speed`,
    };
  }

  const shouldSeek = nowMs - lastCorrectionAtMs >= MIN_CORRECTION_INTERVAL_MS;

  return {
    kind: 'apply',
    playbackIntent: current.status === 'playing' ? undefined : 'play',
    playbackRatePercent: NORMAL_RATE_PERCENT,
    seekToMs: shouldSeek ? heartbeat.positionMs : undefined,
    correctionAtMs: shouldSeek ? nowMs : undefined,
    logMessage: shouldSeek
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

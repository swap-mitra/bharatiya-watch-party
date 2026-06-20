import test from 'node:test';
import assert from 'node:assert/strict';

import {
  HOST_HEARTBEAT_INTERVAL_MS,
  buildHostHeartbeat,
  resolveHeartbeatPlan,
} from './playbackSync.ts';
import type { PlayerState, RoomSession } from './types.ts';

const baseState = (overrides: Partial<PlayerState> = {}): PlayerState => ({
  status: 'playing',
  activeSource: 'https://example.com/video.mp4',
  positionMs: 10_000,
  durationMs: 60_000,
  volume: 100,
  muted: false,
  playbackRatePercent: 100,
  selectedAudioTrack: null,
  selectedSubtitleTrack: null,
  lastError: null,
  ...overrides,
});

const hostSession: RoomSession = {
  roomCode: 'ABC123',
  sessionId: 'host-session',
  role: 'host',
  maxViewers: 10,
};

const viewerSession: RoomSession = {
  roomCode: 'ABC123',
  sessionId: 'viewer-session',
  role: 'viewer',
  maxViewers: 10,
};

test('buildHostHeartbeat returns null when not the host', () => {
  const heartbeat = buildHostHeartbeat({
    session: viewerSession,
    state: baseState(),
    commandSeq: 1,
    nowMs: 1_000,
  });
  assert.equal(heartbeat, null);
});

test('buildHostHeartbeat returns null when there is no active source', () => {
  const heartbeat = buildHostHeartbeat({
    session: hostSession,
    state: baseState({ activeSource: null }),
    commandSeq: 1,
    nowMs: 1_000,
  });
  assert.equal(heartbeat, null);
});

test('buildHostHeartbeat returns null while the player is idle', () => {
  const heartbeat = buildHostHeartbeat({
    session: hostSession,
    state: baseState({ status: 'idle' }),
    commandSeq: 1,
    nowMs: 1_000,
  });
  assert.equal(heartbeat, null);
});

test('buildHostHeartbeat emits a payload while playing', () => {
  const heartbeat = buildHostHeartbeat({
    session: hostSession,
    state: baseState({ status: 'playing', positionMs: 12_500 }),
    commandSeq: 3,
    nowMs: 2_000,
  });
  assert.ok(heartbeat);
  assert.equal(heartbeat?.commandSeq, 3);
  assert.equal(heartbeat?.positionMs, 12_500);
  assert.equal(heartbeat?.status, 'playing');
  assert.equal(heartbeat?.sentAtMs, 2_000);
});

test('resolveHeartbeatPlan ignores heartbeats for hosts', () => {
  const plan = resolveHeartbeatPlan({
    current: baseState(),
    heartbeat: {
      commandSeq: 1,
      positionMs: 10_000,
      status: 'playing',
      activeSource: 'https://example.com/video.mp4',
      sentAtMs: 1_000,
    },
    isHost: true,
    lastCommandSeq: 0,
    lastCorrectionAtMs: 0,
    nowMs: 2_000,
  });
  assert.equal(plan.kind, 'ignore');
});

test('resolveHeartbeatPlan returns a load_source action when source differs', () => {
  const plan = resolveHeartbeatPlan({
    current: baseState({ activeSource: null }),
    heartbeat: {
      commandSeq: 2,
      positionMs: 5_000,
      status: 'playing',
      activeSource: 'https://example.com/video.mp4',
      sentAtMs: 1_000,
    },
    isHost: false,
    lastCommandSeq: 0,
    lastCorrectionAtMs: 0,
    nowMs: 1_000,
  });
  assert.equal(plan.kind, 'apply');
  if (plan.kind === 'apply') {
    assert.equal(plan.loadSource, 'https://example.com/video.mp4');
    assert.equal(plan.seekToMs, 5_000);
    assert.equal(plan.playbackIntent, 'play');
  }
});

test('resolveHeartbeatPlan returns noop for drift under the playing threshold', () => {
  const plan = resolveHeartbeatPlan({
    current: baseState({ positionMs: 10_100, status: 'playing' }),
    heartbeat: {
      commandSeq: 3,
      positionMs: 10_000,
      status: 'playing',
      activeSource: 'https://example.com/video.mp4',
      sentAtMs: 1_000,
    },
    isHost: false,
    lastCommandSeq: 0,
    lastCorrectionAtMs: 0,
    nowMs: 1_000,
  });
  assert.equal(plan.kind, 'apply');
  if (plan.kind === 'apply') {
    assert.equal(plan.seekToMs, undefined);
    assert.equal(plan.playbackRatePercent, undefined);
  }
});

test('resolveHeartbeatPlan smooths moderate drift with playback rate', () => {
  const plan = resolveHeartbeatPlan({
    current: baseState({ positionMs: 11_500, status: 'playing' }),
    heartbeat: {
      commandSeq: 4,
      positionMs: 10_000,
      status: 'playing',
      activeSource: 'https://example.com/video.mp4',
      sentAtMs: 1_000,
    },
    isHost: false,
    lastCommandSeq: 0,
    lastCorrectionAtMs: 0,
    nowMs: 1_000,
  });
  assert.equal(plan.kind, 'apply');
  if (plan.kind === 'apply') {
    assert.equal(plan.seekToMs, undefined);
    assert.equal(plan.playbackRatePercent, 97);
    assert.match(plan.logMessage ?? '', /Smoothing/);
  }
});

test('resolveHeartbeatPlan hard-seeks when drift exceeds the hard threshold', () => {
  const plan = resolveHeartbeatPlan({
    current: baseState({ positionMs: 16_000, status: 'playing' }),
    heartbeat: {
      commandSeq: 5,
      positionMs: 10_000,
      status: 'playing',
      activeSource: 'https://example.com/video.mp4',
      sentAtMs: 1_000,
    },
    isHost: false,
    lastCommandSeq: 0,
    lastCorrectionAtMs: 0,
    nowMs: 1_000,
  });
  assert.equal(plan.kind, 'apply');
  if (plan.kind === 'apply') {
    assert.equal(plan.seekToMs, 10_000);
    assert.equal(plan.playbackRatePercent, 100);
    assert.match(plan.logMessage ?? '', /Hard synced/);
  }
});

test('resolveHeartbeatPlan enforces the correction throttle interval', () => {
  const plan = resolveHeartbeatPlan({
    current: baseState({ positionMs: 14_000, status: 'playing' }),
    heartbeat: {
      commandSeq: 6,
      positionMs: 10_000,
      status: 'playing',
      activeSource: 'https://example.com/video.mp4',
      sentAtMs: 1_000,
    },
    isHost: false,
    lastCommandSeq: 0,
    lastCorrectionAtMs: 999,
    nowMs: 1_000,
  });
  assert.equal(plan.kind, 'apply');
  if (plan.kind === 'apply') {
    assert.equal(plan.seekToMs, undefined);
    assert.equal(plan.playbackRatePercent, 100);
  }
});

test('HOST_HEARTBEAT_INTERVAL_MS is exposed as a public constant', () => {
  assert.ok(HOST_HEARTBEAT_INTERVAL_MS > 0);
});

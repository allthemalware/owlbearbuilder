// SPDX-License-Identifier: GPL-3.0-or-later
'use strict';

(function(global) {
const RELAY_CHANNEL = 'com.owlbearbuilder/relay';
const AOE_KEY = 'com.owlbearbuilder/aoe';
const relayActions = ['import_character', 'roll', 'character_update', 'initiative_roll'];
const listeners = new Map();
const pending = new Map();
let detailsCache;
let connected = false;
let ref = '';
let userId = '';
let nonceCounter = 0;
let idCounter = 0;
let relaySubscribed = false;
const relayHandlers = new Set();
const relayPending = new Map();

function decodeRef(value) {
  try {
    const normalized = value.replace(/-/g, '+').replace(/_/g, '/');
    const padded = normalized + '='.repeat((4 - normalized.length % 4) % 4);
    const decoded = global.atob(padded);
    const text = decodeURIComponent(
        Array.prototype.map
            .call(decoded, (c) => '%' + c.charCodeAt(0).toString(16).padStart(2, '0'))
            .join(''));
    const split = text.indexOf(' ');
    if (split <= 0 || !text.slice(0, split).startsWith('https://')) return null;
    const originMatch = text.slice(0, split).match(/^(https:\/\/[^/]+)/);
    if (!originMatch) return null;
    const origin = originMatch[1];
    return {origin, roomId: text.slice(split + 1)};
  } catch (_) {
    return null;
  }
}

function details() {
  if (detailsCache) return {...detailsCache};
  const params = new URLSearchParams(global.location.search || '');
  detailsCache = decodeRef(params.get('obrref') || '') || {origin: '', roomId: ''};
  return {...detailsCache};
}

function emit(id, data) {
  const callbacks = listeners.get(id);
  if (callbacks) Array.from(callbacks).forEach((fn) => fn(data));
}

function receive(event) {
  const info = details();
  if (!info.origin || event.origin !== info.origin || !event.data ||
      typeof event.data.id !== 'string')
    return;
  const message = event.data;
  const result = pending.get(message.id);
  if (result) {
    pending.delete(result.responseId);
    pending.delete(result.errorId);
    global.clearTimeout(result.timer);
    if (message.id.includes('_ERROR'))
      result.reject(new Error((message.data && message.data.error) || 'Owlbear request failed'));
    else
      result.resolve(message.data);
    return;
  }
  emit(message.id, message.data);
}

global.addEventListener('message', receive);

function connect(timeoutMs) {
  const info = details();
  if (!info.origin) return Promise.resolve({inOwlbear: false});
  if (connected) return Promise.resolve({inOwlbear: true, userId, roomId: info.roomId});
  return new Promise((resolve) => {
    let settled = false;
    const finish = (value) => {
      if (settled) return;
      settled = true;
      global.clearTimeout(timer);
      unsubscribe();
      resolve(value);
    };
    const unsubscribe = on('OBR_READY', (data) => {
      if (!data || typeof data.ref !== 'string' || !data.ref) return;
      ref = data.ref;
      userId = data.userId || '';
      connected = true;
      finish({inOwlbear: true, userId, roomId: info.roomId});
    });
    const timer = global.setTimeout(
        () => finish({inOwlbear: false}), timeoutMs === undefined ? 3000 : timeoutMs);
  });
}

function on(id, fn) {
  if (!listeners.has(id)) listeners.set(id, new Set());
  const callbacks = listeners.get(id);
  callbacks.add(fn);
  return () => {
    callbacks.delete(fn);
    if (!callbacks.size) listeners.delete(id);
  };
}

function call(id, data, timeoutMs) {
  if (!connected) return Promise.reject(new Error('Owlbear is not connected'));
  const token = '_' + Date.now().toString(36) + (++nonceCounter).toString(36);
  const responseId = id + '_RESPONSE' + token;
  const errorId = id + '_ERROR' + token;
  return new Promise((resolve, reject) => {
    const timer = global.setTimeout(() => {
      pending.delete(responseId);
      pending.delete(errorId);
      reject(new Error('Owlbear request times out'));
    }, timeoutMs === undefined ? 5000 : timeoutMs);
    const waiter = {resolve, reject, timer, responseId, errorId};
    pending.set(responseId, waiter);
    pending.set(errorId, waiter);
    global.parent.postMessage({id, data: data || {}, ref, nonce: token}, details().origin);
  });
}

function send(id, data) {
  if (!connected) return Promise.reject(new Error('Owlbear is not connected'));
  global.parent.postMessage({id, data: data || {}, ref}, details().origin);
  return Promise.resolve();
}

function getRole() {
  return call('OBR_PLAYER_GET_ROLE', {}).then((data) => data && data.role);
}
function getSelection() {
  return call('OBR_PLAYER_GET_SELECTION', {});
}
function getGridDpi() {
  return call('OBR_SCENE_GRID_GET', {}).then((data) => data && data.dpi);
}
function getItems(ids) {
  return call('OBR_SCENE_ITEMS_GET_ITEMS', {ids: ids || []})
      .then((data) => data && data.items || []);
}
function addItems(items) {
  return call('OBR_SCENE_ITEMS_ADD_ITEMS', {items});
}
function deleteItems(ids) {
  return call('OBR_SCENE_ITEMS_DELETE_ITEMS', {items: ids});
}
async function setItemMetadata(ids, key, value) {
  const items = await getItems(ids);
  const updates = items.map((item) => ({
                              id: item.id,
                              type: item.type,
                              metadata: {...(item.metadata || {}), [key]: value},
                            }));
  if (updates.length) return call('OBR_SCENE_ITEMS_UPDATE_ITEMS', {updates});
  return undefined;
}
function getRoomMetadata() {
  return call('OBR_ROOM_GET_METADATA', {}).then((data) => data && data.metadata || {});
}
function onRoomMetadata(fn) {
  const unsubscribe = on('OBR_ROOM_METADATA_EVENT_CHANGE', fn);
  send('OBR_ROOM_METADATA_SUBSCRIBE', {}).catch(() => {});
  return unsubscribe;
}
function setRoomMetadata(metadata) {
  return call('OBR_ROOM_SET_METADATA', {metadata});
}
function broadcast(channel, data) {
  return call('OBR_BROADCAST_SEND_MESSAGE', {channel, data, options: {}});
}
function onBroadcast(channel, fn) {
  return on('OBR_BROADCAST_MESSAGE_' + channel, fn);
}
function notify(message, variant) {
  return call('OBR_NOTIFICATION_SHOW', {message, variant: variant || 'INFO'});
}
function createContextMenu(id, items) {
  return call('OBR_CONTEXT_MENU_CREATE', {id, items});
}
function onContextMenuClick(id, fn) {
  return on('OBR_CONTEXT_MENU_CLICK_' + id, fn);
}

function uniqueId() {
  idCounter += 1;
  if (global.crypto && global.crypto.randomUUID) return global.crypto.randomUUID();
  return 'obb-' + Date.now().toString(36) + '-' + idCounter.toString(36);
}
function itemBase(descriptor, type) {
  const now = new Date().toISOString();
  return {
    id: uniqueId(),
    type,
    name: descriptor.label || 'Area of Effect',
    position: {x: descriptor.position.x, y: descriptor.position.y},
    rotation: 0,
    scale: {x: 1, y: 1},
    metadata: {...(descriptor.metadata || {})},
    layer: 'DRAWING',
    visible: true,
    locked: false,
    zIndex: 0,
    createdUserId: userId,
    lastModified: now,
    lastModifiedUserId: userId,
  };
}
function aoeToItems(descriptor) {
  const style = {
    fillColor: descriptor.fill_color,
    fillOpacity: descriptor.fill_opacity,
    strokeColor: descriptor.stroke_color,
    strokeOpacity: descriptor.stroke_opacity === undefined ? 1 : descriptor.stroke_opacity,
    strokeWidth: descriptor.stroke_width === undefined ? 2 : descriptor.stroke_width,
  };
  if (descriptor.shape_type === 'CIRCLE') {
    return [{
      ...itemBase(descriptor, 'SHAPE'),
      shapeType: 'CIRCLE',
      width: descriptor.width,
      height: descriptor.height,
      style
    }];
  }
  if (descriptor.shape_type === 'POLYGON') {
    const position = descriptor.position;
    return [{
      ...itemBase(descriptor, 'CURVE'),
      position: {x: position.x, y: position.y},
      points: (descriptor.points || []).map((point) => ({x: point.x, y: point.y})),
      style: {...style, closed: true, tension: 0},
    }];
  }
  return [];
}
function placeAoe(descriptor) {
  return addItems(aoeToItems(descriptor));
}
async function clearAoe() {
  const items = await getItems([]);
  const ids =
      items.filter((item) => item.metadata && item.metadata[AOE_KEY]).map((item) => item.id);
  if (ids.length) return deleteItems(ids);
  return undefined;
}

function ensureRelaySubscription() {
  if (relaySubscribed) return;
  relaySubscribed = true;
  onBroadcast(RELAY_CHANNEL, receiveRelay);
  call('OBR_BROADCAST_SUBSCRIBE', {channel: RELAY_CHANNEL}).catch(() => {});
}
function relaySend(message) {
  ensureRelaySubscription();
  return broadcast(RELAY_CHANNEL, message);
}
function receiveRelay(event) {
  const message = event && event.data && event.data.type ? event.data : event;
  if (!message || typeof message !== 'object') return;
  if (message.type === 'reply') {
    const waiter = relayPending.get(message.requestId);
    if (!waiter) return;
    relayPending.delete(message.requestId);
    global.clearTimeout(waiter.timer);
    if (message.error)
      waiter.reject(new Error(message.error));
    else
      waiter.resolve(message.result);
    return;
  }
  if (message.type !== 'request' || !relayActions.includes(message.action)) return;
  relayHandlers.forEach(async (handler) => {
    try {
      const result = await handler(
          message.action, message.payload,
          event && (event.fromUserId || event.userId || event.senderId || event.connectionId));
      relaySend({type: 'reply', requestId: message.requestId, result});
    } catch (error) {
      relaySend({
        type: 'reply',
        requestId: message.requestId,
        error: error && error.message || 'Request failed'
      });
    }
  });
}
const relay = {
  ACTIONS: relayActions.slice(),
  serve(handler) {
    ensureRelaySubscription();
    relayHandlers.add(handler);
    return () => relayHandlers.delete(handler);
  },
  request(action, payload, timeoutMs) {
    if (!relayActions.includes(action))
      return Promise.reject(new Error('only whitelisted actions are relayed: ' + action));
    const requestId = uniqueId();
    ensureRelaySubscription();
    return new Promise((resolve, reject) => {
      const timer = global.setTimeout(() => {
        relayPending.delete(requestId);
        reject(new Error('relay request times out'));
      }, timeoutMs === undefined ? 10000 : timeoutMs);
      relayPending.set(requestId, {resolve, reject, timer});
      relaySend({type: 'request', requestId, action, payload, fromUserId: userId})
          .catch((error) => {
            const waiter = relayPending.get(requestId);
            if (waiter) {
              relayPending.delete(requestId);
              global.clearTimeout(waiter.timer);
              reject(error);
            }
          });
    });
  },
};

function apiBase() {
  try {
    const saved = global.localStorage.getItem('obb.apiBase');
    return (saved || 'http://127.0.0.1:8080').replace(/\/+$/, '');
  } catch (_) {
    return 'http://127.0.0.1:8080';
  }
}
function setApiBase(url) {
  const value = String(url).replace(/\/+$/, '');
  global.localStorage.setItem('obb.apiBase', value);
  return value;
}
async function api(path, body) {
  const options = body === undefined ?
      {} :
      {method: 'POST', headers: {'Content-Type': 'application/json'}, body: JSON.stringify(body)};
  const response = await global.fetch(apiBase() + path, options);
  const data = await response.json();
  if (!response.ok) throw new Error(data.error || 'Request failed');
  return data;
}
function events(onState) {
  const source = new global.EventSource(apiBase() + '/api/events');
  source.addEventListener('state', (event) => onState(JSON.parse(event.data)));
  return source;
}

global.ObrBridge = {
  details,
  connect,
  call,
  on,
  getRole,
  getSelection,
  getGridDpi,
  getItems,
  addItems,
  deleteItems,
  setItemMetadata,
  getRoomMetadata,
  onRoomMetadata,
  setRoomMetadata,
  send,
  broadcast,
  onBroadcast,
  notify,
  createContextMenu,
  onContextMenuClick,
  aoeToItems,
  placeAoe,
  clearAoe,
  relay,
  apiBase,
  setApiBase,
  api,
  events,
};
})(window);

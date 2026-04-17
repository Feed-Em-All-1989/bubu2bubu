const { invoke } = window.__TAURI__.core

const $ = (s) => document.querySelector(s)
const messages = $('#messages')

let polling = false
let imageMode = false
let replyTo = null
let messageMap = {}
let messageOrder = []
let disconnected = false
let reconnecting = false
let userPollInterval = null
let peerProfiles = {}

async function init() {
  const tag = await invoke('get_key_tag')
  $('#key-tag').textContent = '#' + tag

  const savedName = await invoke('get_saved_username')
  if (savedName) $('#name-input').value = savedName

  const avatar = await invoke('get_avatar')
  if (avatar) {
    $('#own-avatar').src = 'data:image/png;base64,' + avatar
  }
}

function formatDate(ts) {
  const d = new Date(ts * 1000)
  const now = new Date()
  const today = new Date(now.getFullYear(), now.getMonth(), now.getDate())
  const msgDay = new Date(d.getFullYear(), d.getMonth(), d.getDate())
  const diff = (today - msgDay) / 86400000
  if (diff === 0) return 'today'
  if (diff === 1) return 'yesterday'
  return d.toLocaleDateString([], { month: 'short', day: 'numeric', year: 'numeric' })
}

function formatTime(ts) {
  return new Date(ts * 1000).toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' })
}

function dayKey(ts) {
  const d = new Date(ts * 1000)
  return `${d.getFullYear()}-${d.getMonth()}-${d.getDate()}`
}

function createDateSeparator(ts) {
  const div = document.createElement('div')
  div.className = 'date-separator'
  div.textContent = formatDate(ts)
  return div
}

function buildMessageEl(msg) {
  const div = document.createElement('div')
  div.className = `message ${msg.from_self ? 'self' : 'peer'}`
  if (msg.sender === 'system') div.className = 'message system'
  div.dataset.id = msg.id

  if (msg.sender !== 'system' && !msg.from_self) {
    const senderRow = document.createElement('div')
    senderRow.className = 'sender-row'
    const profile = peerProfiles[msg.sender]
    const ava = document.createElement('img')
    ava.className = 'msg-avatar'
    ava.src = (profile && profile.avatar) ? 'data:image/png;base64,' + profile.avatar : 'default-avatar.png'
    if (profile) ava.addEventListener('click', (e) => { e.stopPropagation(); showProfilePopup(profile) })
    senderRow.appendChild(ava)
    const sender = document.createElement('span')
    sender.className = 'sender'
    sender.textContent = msg.sender
    senderRow.appendChild(sender)
    div.appendChild(senderRow)
  }

  if (msg.reply_to && messageMap[msg.reply_to]) {
    const quote = document.createElement('div')
    quote.className = 'quote'
    const orig = messageMap[msg.reply_to]
    quote.textContent = (orig.sender || 'you') + ': ' + orig.content.slice(0, 80)
    div.appendChild(quote)
  }

  const body = document.createElement('div')
  body.className = 'msg-body'

  if (imageMode && msg.stego_image) {
    const img = document.createElement('img')
    img.className = 'stego-img'
    img.src = 'data:image/png;base64,' + msg.stego_image
    body.appendChild(img)
  } else {
    const text = document.createElement('span')
    text.textContent = msg.content
    body.appendChild(text)
  }

  div.appendChild(body)

  const time = document.createElement('span')
  time.className = 'time'
  time.textContent = formatTime(msg.timestamp)
  div.appendChild(time)

  if (msg.sender !== 'system') {
    div.addEventListener('click', () => {
      replyTo = msg.id
      const preview = (msg.sender || 'you') + ': ' + msg.content.slice(0, 60)
      $('#reply-preview').textContent = preview
      $('#reply-bar').classList.remove('hidden')
      $('#msg-input').focus()
    })
  }

  return div
}

function addMessage(msg) {
  messageMap[msg.id] = msg
  const prevMsg = messageOrder.length > 0 ? messageMap[messageOrder[messageOrder.length - 1]] : null
  messageOrder.push(msg.id)

  if (!prevMsg || dayKey(prevMsg.timestamp) !== dayKey(msg.timestamp)) {
    messages.appendChild(createDateSeparator(msg.timestamp))
  }

  messages.appendChild(buildMessageEl(msg))
  messages.scrollTop = messages.scrollHeight
}

function rebuildMessages() {
  messages.innerHTML = ''
  let lastDay = null
  for (const id of messageOrder) {
    const msg = messageMap[id]
    const dk = dayKey(msg.timestamp)
    if (dk !== lastDay) {
      messages.appendChild(createDateSeparator(msg.timestamp))
      lastDay = dk
    }
    messages.appendChild(buildMessageEl(msg))
  }
  messages.scrollTop = messages.scrollHeight
}

async function refreshUserList() {
  try {
    const users = await invoke('get_online_users')
    const profiles = await invoke('get_peer_profiles')
    peerProfiles = {}
    for (const p of profiles) peerProfiles[p.name] = p

    const list = $('#user-list')
    list.innerHTML = ''
    for (const name of users) {
      const el = document.createElement('div')
      el.className = 'user-entry'
      const profile = peerProfiles[name]
      const ava = document.createElement('img')
      ava.className = 'user-avatar'
      ava.src = (profile && profile.avatar) ? 'data:image/png;base64,' + profile.avatar : 'default-avatar.png'
      if (profile) ava.addEventListener('click', (e) => { e.stopPropagation(); showProfilePopup(profile) })
      el.appendChild(ava)
      const label = document.createElement('span')
      label.textContent = name
      el.appendChild(label)
      list.appendChild(el)
    }
    $('#user-list-label').textContent = `online (${users.length})`
  } catch (_) {}
}

function startUserPolling() {
  if (userPollInterval) return
  refreshUserList()
  userPollInterval = setInterval(refreshUserList, 2000)
}

function stopUserPolling() {
  if (userPollInterval) {
    clearInterval(userPollInterval)
    userPollInterval = null
  }
}

function showDisconnect() {
  disconnected = true
  $('#status-dot').className = 'offline'
  $('#disconnect-banner').classList.remove('hidden')
  $('#disconnect-text').textContent = 'disconnected from server'
  $('#msg-input').disabled = true
  $('#btn-send').disabled = true
  stopUserPolling()
}

function hideDisconnect() {
  disconnected = false
  $('#status-dot').className = 'online'
  $('#disconnect-banner').classList.add('hidden')
  $('#msg-input').disabled = false
  $('#btn-send').disabled = false
  startUserPolling()
}

async function attemptReconnect() {
  if (reconnecting) return
  reconnecting = true
  $('#disconnect-text').textContent = 'reconnecting...'
  $('#btn-reconnect').disabled = true

  try {
    await invoke('reconnect')
    hideDisconnect()
    addMessage({
      id: Date.now().toString(),
      sender: 'system',
      content: 'reconnected to server',
      from_self: false,
      timestamp: Math.floor(Date.now() / 1000),
      reply_to: null,
      stego_image: null
    })
    reconnecting = false
    startPolling()
  } catch (e) {
    reconnecting = false
    $('#disconnect-text').textContent = 'reconnect failed, retrying...'
    $('#btn-reconnect').disabled = false
    await new Promise((r) => setTimeout(r, 3000))
    if (disconnected) attemptReconnect()
  }
}

async function startPolling() {
  if (polling) return
  polling = true
  while (polling) {
    try {
      const msg = await invoke('recv_message')
      addMessage(msg)
      if (msg.sender === 'system') refreshUserList()
    } catch (e) {
      if (e === 'disconnected') {
        polling = false
        showDisconnect()
        addMessage({
          id: Date.now().toString(),
          sender: 'system',
          content: 'lost connection to server',
          from_self: false,
          timestamp: Math.floor(Date.now() / 1000),
          reply_to: null,
          stego_image: null
        })
        await new Promise((r) => setTimeout(r, 2000))
        attemptReconnect()
        return
      }
      if (e !== 'timeout') {
        addMessage({
          id: Date.now().toString(),
          sender: 'system',
          content: '[error] ' + e,
          from_self: false,
          timestamp: Math.floor(Date.now() / 1000),
          reply_to: null,
          stego_image: null
        })
      }
      await new Promise((r) => setTimeout(r, 100))
    }
  }
}

let pendingRoomKey = null
let connectedName = null

$('#btn-connect').addEventListener('click', async () => {
  const name = $('#name-input').value.trim()
  const addr = $('#addr-input').value.trim()
  if (!name || !addr) return

  try {
    $('#btn-connect').textContent = 'connecting...'
    $('#btn-connect').disabled = true
    await invoke('set_username', { name })
    const roomKey = await invoke('connect_to_server', { addr })
    connectedName = name
    pendingRoomKey = roomKey
    $('#key-prompt-value').textContent = roomKey
    $('#key-prompt-input').value = ''
    $('#key-prompt-overlay').classList.remove('hidden')
  } catch (e) {
    $('#btn-connect').textContent = 'connect'
    $('#btn-connect').disabled = false
    addMessage({
      id: Date.now().toString(),
      sender: 'system',
      content: '[connect error] ' + e,
      from_self: false,
      timestamp: Math.floor(Date.now() / 1000),
      reply_to: null,
      stego_image: null
    })
  }
})

function finishConnect(activeKey) {
  $('#key-prompt-overlay').classList.add('hidden')
  $('#status-dot').className = 'online'
  $('#connect-panel').classList.add('hidden')
  $('#server-info').classList.remove('hidden')
  $('#server-addr-label').textContent = 'connected as ' + connectedName
  $('#stego-key-value').textContent = activeKey
  startPolling()
  startUserPolling()
}

$('#btn-accept-key').addEventListener('click', async () => {
  finishConnect(pendingRoomKey)
})

$('#btn-use-custom-key').addEventListener('click', async () => {
  const custom = $('#key-prompt-input').value.trim()
  if (!custom) return
  await invoke('set_encryption_key', { key: custom })
  finishConnect(custom)
})

$('#btn-reconnect').addEventListener('click', () => {
  attemptReconnect()
})

$('#btn-copy-key').addEventListener('click', () => {
  const key = $('#stego-key-value').textContent
  navigator.clipboard.writeText(key).catch(() => {})
  $('#btn-copy-key').textContent = 'copied'
  setTimeout(() => { $('#btn-copy-key').textContent = 'copy' }, 1500)
})

$('#btn-send').addEventListener('click', sendMessage)
$('#msg-input').addEventListener('keydown', (e) => {
  if (e.key === 'Enter') sendMessage()
})

async function sendMessage() {
  const input = $('#msg-input')
  const text = input.value.trim()
  if (!text) return

  input.value = ''
  const currentReply = replyTo
  clearReply()

  try {
    const msg = await invoke('send_message', { text, replyTo: currentReply })
    addMessage(msg)
  } catch (e) {
    if (e === 'disconnected' || e.includes('not connected')) {
      showDisconnect()
      attemptReconnect()
      return
    }
    addMessage({
      id: Date.now().toString(),
      sender: 'system',
      content: '[send error] ' + e,
      from_self: false,
      timestamp: Math.floor(Date.now() / 1000),
      reply_to: null,
      stego_image: null
    })
  }
}

function clearReply() {
  replyTo = null
  $('#reply-bar').classList.add('hidden')
  $('#reply-preview').textContent = ''
}

$('#btn-cancel-reply').addEventListener('click', clearReply)

$('#btn-labubu').addEventListener('click', () => {
  imageMode = !imageMode
  $('#btn-labubu').classList.toggle('active', imageMode)
  rebuildMessages()
})

$('#btn-rename').addEventListener('click', async () => {
  const newName = prompt('new username')
  if (!newName || !newName.trim()) return
  try {
    await invoke('update_username', { name: newName.trim() })
    connectedName = newName.trim()
    $('#server-addr-label').textContent = 'connected as ' + connectedName
  } catch (e) {
    addMessage({
      id: Date.now().toString(),
      sender: 'system',
      content: '[rename error] ' + e,
      from_self: false,
      timestamp: Math.floor(Date.now() / 1000),
      reply_to: null,
      stego_image: null
    })
  }
})

$('#btn-change-avatar').addEventListener('click', () => {
  $('#avatar-input').click()
})

$('#avatar-input').addEventListener('change', async (e) => {
  const file = e.target.files[0]
  if (!file) return
  const reader = new FileReader()
  reader.onload = async () => {
    const base64 = reader.result.split(',')[1]
    try {
      const sanitized = await invoke('set_avatar', { data: base64 })
      $('#own-avatar').src = 'data:image/png;base64,' + sanitized
    } catch (err) {
      addMessage({
        id: Date.now().toString(),
        sender: 'system',
        content: '[avatar error] ' + err,
        from_self: false,
        timestamp: Math.floor(Date.now() / 1000),
        reply_to: null,
        stego_image: null
      })
    }
  }
  reader.readAsDataURL(file)
  e.target.value = ''
})

function showProfilePopup(profile) {
  if (profile.avatar) {
    $('#popup-avatar').src = 'data:image/png;base64,' + profile.avatar
  } else {
    $('#popup-avatar').src = 'default-avatar.png'
  }
  $('#popup-name').textContent = profile.name
  $('#popup-tag').textContent = '#' + profile.key_tag
  $('#profile-popup').classList.remove('hidden')
}

$('#btn-close-popup').addEventListener('click', () => {
  $('#profile-popup').classList.add('hidden')
})

$('#profile-popup').addEventListener('click', (e) => {
  if (e.target === $('#profile-popup')) $('#profile-popup').classList.add('hidden')
})

init()

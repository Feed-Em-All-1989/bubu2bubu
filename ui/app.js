const { invoke } = window.__TAURI__.core

const $ = (s) => document.querySelector(s)
const messages = $('#messages')

let polling = false
let imageMode = false
let replyTo = null
let messageMap = {}

async function init() {
  const key = await invoke('get_public_key')
  $('#pubkey').textContent = key
}

function addMessage(msg) {
  messageMap[msg.id] = msg

  const div = document.createElement('div')
  div.className = `message ${msg.from_self ? 'self' : 'peer'}`
  if (msg.sender === 'system') div.className = 'message system'
  div.dataset.id = msg.id

  if (msg.sender !== 'system' && !msg.from_self) {
    const sender = document.createElement('span')
    sender.className = 'sender'
    sender.textContent = msg.sender
    div.appendChild(sender)
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
  const d = new Date(msg.timestamp * 1000)
  time.textContent = d.toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' })
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

  messages.appendChild(div)
  messages.scrollTop = messages.scrollHeight
}

function refreshMessages() {
  messages.innerHTML = ''
  for (const id in messageMap) {
    addMessageDom(messageMap[id])
  }
}

function addMessageDom(msg) {
  const div = document.createElement('div')
  div.className = `message ${msg.from_self ? 'self' : 'peer'}`
  if (msg.sender === 'system') div.className = 'message system'
  div.dataset.id = msg.id

  if (msg.sender !== 'system' && !msg.from_self) {
    const sender = document.createElement('span')
    sender.className = 'sender'
    sender.textContent = msg.sender
    div.appendChild(sender)
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
  const d = new Date(msg.timestamp * 1000)
  time.textContent = d.toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' })
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

  messages.appendChild(div)
}

async function startPolling() {
  if (polling) return
  polling = true
  while (polling) {
    try {
      const msg = await invoke('recv_message')
      addMessage(msg)
    } catch (e) {
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

$('#btn-connect').addEventListener('click', async () => {
  const name = $('#name-input').value.trim()
  const addr = $('#addr-input').value.trim()
  if (!name || !addr) return

  try {
    $('#btn-connect').textContent = 'connecting...'
    $('#btn-connect').disabled = true
    await invoke('set_username', { name })
    await invoke('connect_to_server', { addr })
    $('#status-dot').className = 'online'
    $('#connect-panel').classList.add('hidden')
    $('#server-info').classList.remove('hidden')
    $('#server-addr-label').textContent = 'connected as ' + name
    startPolling()
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
  messages.innerHTML = ''
  for (const id in messageMap) {
    addMessageDom(messageMap[id])
  }
  messages.scrollTop = messages.scrollHeight
})

init()

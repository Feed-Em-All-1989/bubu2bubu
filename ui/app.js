const { invoke } = window.__TAURI__.core

const $ = (s) => document.querySelector(s)
const messages = $('#messages')

let polling = false

async function init() {
  const key = await invoke('get_public_key')
  $('#pubkey').textContent = key
}

function addMessage(content, fromSelf, timestamp) {
  const div = document.createElement('div')
  div.className = `message ${fromSelf ? 'self' : 'peer'}`

  const text = document.createElement('span')
  text.textContent = content

  const time = document.createElement('span')
  time.className = 'time'
  const d = new Date(timestamp * 1000)
  time.textContent = d.toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' })

  div.appendChild(text)
  div.appendChild(time)
  messages.appendChild(div)
  messages.scrollTop = messages.scrollHeight
}

async function startPolling() {
  if (polling) return
  polling = true
  while (polling) {
    try {
      const msg = await invoke('recv_message')
      addMessage(msg.content, false, msg.timestamp)
    } catch (e) {
      if (e !== 'timeout') {
        addMessage('[error] ' + e, false, Math.floor(Date.now() / 1000))
      }
      await new Promise((r) => setTimeout(r, 100))
    }
  }
}

$('#btn-host').addEventListener('click', async () => {
  const port = parseInt($('#port-input').value) || 9999
  try {
    $('#btn-host').textContent = 'waiting...'
    $('#btn-host').disabled = true
    await invoke('host_session', { port })
    $('#status-dot').className = 'online'
    $('#connect-panel').classList.add('hidden')
    $('#peer-info').classList.remove('hidden')
    startPolling()
  } catch (e) {
    $('#btn-host').textContent = 'host'
    $('#btn-host').disabled = false
    console.error(e)
  }
})

$('#btn-join').addEventListener('click', async () => {
  const addr = $('#addr-input').value.trim()
  if (!addr) return
  try {
    $('#btn-join').textContent = 'connecting...'
    $('#btn-join').disabled = true
    await invoke('join_session', { addr })
    $('#status-dot').className = 'online'
    $('#connect-panel').classList.add('hidden')
    $('#peer-info').classList.remove('hidden')
    startPolling()
  } catch (e) {
    $('#btn-join').textContent = 'join'
    $('#btn-join').disabled = false
    alert('Join failed: ' + e)
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
  const timestamp = Math.floor(Date.now() / 1000)
  addMessage(text, true, timestamp)
 
  try {
    await invoke('send_message', { text })
  } catch (e) {
    addMessage('[send error] ' + e, true, Math.floor(Date.now() / 1000))
  }
}

init()

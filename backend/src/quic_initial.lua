-- Adapted from zemidala/nfqws2-ui, Copyright (c) 2026 zemidala (MIT).
-- See THIRD_PARTY_NOTICES.md for the full license.
local function u8(n) return string.char(n % 256) end
local function u16(n) return string.char(math.floor(n / 256) % 256, n % 256) end
local function u24(n) return string.char(math.floor(n / 65536) % 256, math.floor(n / 256) % 256, n % 256) end
local function v16(b) return u16(#b) .. b end
local function varint(n) if n < 64 then return u8(n) end return u16(0x4000 + n) end

local function hmac(key, msg)
  key = key .. string.rep("\0", 64 - #key)
  return hash("sha256", bxor(key, string.rep("\92", 64)) .. hash("sha256", bxor(key, string.rep("\54", 64)) .. msg))
end
local function expand_label(secret, label, len)
  local l = "tls13 " .. label
  return hmac(secret, u16(len) .. u8(#l) .. l .. u8(0) .. "\1"):sub(1, len)
end

local dcid, scid = brandom(8), brandom(8)

-- ClientHello TLS 1.3: имя, группы, подписи, ALPN h3, версия, ключ x25519, параметры QUIC
local function ext(t, b) return u16(t) .. v16(b) end
local tp = u8(0x0f) .. u8(#scid) .. scid .. u8(0x04) .. u8(4) .. "\128\16\0\0" .. u8(0x01) .. u8(2) .. u16(0x4000 + 13616)
local exts = ext(0, v16(u8(0) .. v16(SNI)))
  .. ext(10, v16(u16(0x001d) .. u16(0x0017)))
  .. ext(13, v16(u16(0x0403) .. u16(0x0804) .. u16(0x0401) .. u16(0x0503) .. u16(0x0805) .. u16(0x0501) .. u16(0x0806) .. u16(0x0601)))
  .. ext(16, v16(u8(2) .. "h3"))
  .. ext(43, u8(2) .. u16(0x0304))
  .. ext(45, u8(1) .. u8(1))
  .. ext(51, v16(u16(0x001d) .. v16(brandom(32))))
  .. ext(57, tp)
local body = u16(0x0303) .. brandom(32) .. u8(0) .. v16(u16(0x1301) .. u16(0x1302) .. u16(0x1303)) .. u8(1) .. u8(0) .. v16(exts)
local ch = u8(1) .. u24(#body) .. body

-- кадр CRYPTO и добивка: дейтаграмма с Initial не короче 1200 байт
local frames = u8(6) .. varint(0) .. varint(#ch) .. ch
local pn_len = 4
local need = 1200 - (1 + 4 + 1 + 8 + 1 + 8 + 1 + 2) - pn_len - 16
if #frames < need then frames = frames .. string.rep("\0", need - #frames) end

-- ключи Initial: соль QUIC v1 и идентификатор соединения
local salt = "\56\118\44\247\245\89\52\179\77\23\154\230\164\200\12\173\204\187\127\10"
local cis = expand_label(hmac(salt, dcid), "client in", 32)
local key, iv, hp = expand_label(cis, "quic key", 16), expand_label(cis, "quic iv", 12), expand_label(cis, "quic hp", 16)

local pn = "\0\0\0\0"
local header = u8(0xC3) .. "\0\0\0\1" .. u8(8) .. dcid .. u8(8) .. scid .. u8(0) .. varint(pn_len + #frames + 16) .. pn
local enc, tag = aes_gcm(true, key, iv, frames, header)   -- номер пакета 0: nonce = iv
local packet = header .. enc .. tag

-- защита заголовка
local pn_off = #header - pn_len
local mask = aes(true, hp, packet:sub(pn_off + 5, pn_off + 20))
packet = u8(bitxor(packet:byte(1), bitand(mask:byte(1), 0x0f))) .. packet:sub(2, pn_off) .. bxor(pn, mask:sub(2, 5)) .. packet:sub(pn_off + pn_len + 1)

local hex = packet:gsub(".", function(c) return string.format("%02x", c:byte()) end)
print("QUICHEX " .. hex)

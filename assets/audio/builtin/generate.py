#!/usr/bin/env python3
"""Generate built-in sound effects for FilmCraft M5.
Outputs WAV files (44.1 kHz, mono, 16-bit) into the same directory.
Run: python3 generate.py
"""
import math
import os
import struct
import sys

import numpy as np

OUT = os.path.dirname(os.path.abspath(__file__))
SAMPLE_RATE = 44100

def write_wav(path, samples, sample_rate=SAMPLE_RATE):
    """samples: float numpy array in [-1, 1], mono."""
    samples = np.clip(samples, -1.0, 1.0)
    pcm = (samples * 32767).astype(np.int16)
    data = pcm.tobytes()
    header = struct.pack('<4sI4s4sIHHIIHH4sI',
        b'RIFF', 36 + len(data), b'WAVE', b'fmt ', 16, 1, 1,
        sample_rate, sample_rate * 2, 2, 16, b'data', len(data))
    with open(path, 'wb') as f:
        f.write(header + data)

def t(dur):
    return np.arange(int(dur * SAMPLE_RATE)) / SAMPLE_RATE

def fade(samples, fade_in=0.01, fade_out=0.05):
    n = len(samples)
    fi = int(fade_in * SAMPLE_RATE)
    fo = int(fade_out * SAMPLE_RATE)
    if fi > 0:
        samples[:fi] *= np.linspace(0, 1, fi)
    if fo > 0:
        samples[-fo:] *= np.linspace(1, 0, fo)
    return samples

def sine(freq, dur, vol=0.5):
    return vol * np.sin(2 * np.pi * freq * t(dur))

def noise(dur, vol=0.3, kind='white'):
    n = int(dur * SAMPLE_RATE)
    if kind == 'white':
        x = np.random.uniform(-1, 1, n)
    elif kind == 'pink':
        # Simple pink noise approximation
        x = np.random.uniform(-1, 1, n)
        x = np.cumsum(x)
        x = x / np.max(np.abs(x)) * vol
        return x
    return vol * x

# ----------------------------------------------------------------------
# 25+ sound effects
# ----------------------------------------------------------------------
sounds = []

# 1. Whoosh (sfx, transitions)
def whoosh():
    dur = 0.8
    x = noise(dur, 0.4)
    # Sweeping filter: lowpass with moving cutoff
    for i in range(len(x)):
        cutoff = 300 + 4000 * (i / len(x)) ** 2
        # simple one-pole lowpass
        alpha = cutoff / (cutoff + SAMPLE_RATE)
        if i > 0:
            x[i] = alpha * x[i] + (1 - alpha) * x[i-1]
    x = fade(x, 0.05, 0.3)
    x *= np.linspace(0, 1, len(x)) ** 2  # swell
    return x
sounds.append(("whoosh-01.wav", "sfx.whoosh.01", "Whoosh 01", "transitions", whoosh(), 0.8, ["whoosh", "переход"]))

# 2. Impact/boom
def impact():
    dur = 0.6
    x = np.zeros(int(dur * SAMPLE_RATE))
    # Low thud
    x += 0.6 * np.sin(2 * np.pi * 60 * t(dur)) * np.exp(-t(dur) * 8)
    # High crack
    x += 0.3 * noise(dur, 0.5) * np.exp(-t(dur) * 30)
    x = fade(x, 0.01, 0.4)
    return x
sounds.append(("impact-01.wav", "sfx.impact.01", "Impact 01", "удары", impact(), 0.6, ["impact", "удар", "boom"]))

# 3. Riser
def riser():
    dur = 1.5
    tt = t(dur)
    freq = 150 * np.exp(tt * 3)  # exponential rise
    phase = 2 * np.pi * np.cumsum(freq) / SAMPLE_RATE
    x = 0.4 * np.sin(phase) * np.exp(-tt * 0.5)
    x = fade(x, 0.1, 0.3)
    return x
sounds.append(("riser-01.wav", "sfx.riser.01", "Riser 01", "райзеры", riser(), 1.5, ["riser", "подъём", "sweep"]))

# 4. Click
def click():
    dur = 0.1
    x = 0.5 * np.sin(2 * np.pi * 1000 * t(dur)) * np.exp(-t(dur) * 100)
    x = fade(x, 0.005, 0.05)
    return x
sounds.append(("click-01.wav", "sfx.click.01", "Click 01", "интерфейс", click(), 0.1, ["click", "интерфейс", "ui"]))

# 5. Transition swish
def swish():
    dur = 0.5
    x = noise(dur, 0.3)
    for i in range(len(x)):
        cutoff = 500 + 3000 * math.sin(i / len(x) * math.pi)
        alpha = cutoff / (cutoff + SAMPLE_RATE)
        if i > 0:
            x[i] = alpha * x[i] + (1 - alpha) * x[i-1]
    x = fade(x, 0.02, 0.2)
    return x
sounds.append(("swish-01.wav", "sfx.swish.01", "Swish 01", "переходы", swish(), 0.5, ["swish", "переход"]))

# 6. Glitch
def glitch():
    dur = 0.3
    x = np.zeros(int(dur * SAMPLE_RATE))
    for i in range(0, len(x), 100):
        end = min(i + 50, len(x))
        x[i:end] = 0.4 * np.random.uniform(-1, 1, end - i)
    x = fade(x, 0.01, 0.1)
    return x
sounds.append(("glitch-01.wav", "sfx.glitch.01", "Glitch 01", "техника", glitch(), 0.3, ["glitch", "цифровой"]))

# 7. Pop
def pop():
    dur = 0.2
    tt = t(dur)
    x = 0.5 * np.sin(2 * np.pi * 800 * tt) * np.exp(-tt * 20)
    x = fade(x, 0.01, 0.1)
    return x
sounds.append(("pop-01.wav", "sfx.pop.01", "Pop 01", "интерфейс", pop(), 0.2, ["pop", "всплытие"]))

# 8. Typing
def typing():
    dur = 0.4
    x = np.zeros(int(dur * SAMPLE_RATE))
    for i in range(0, len(x), 3000):
        end = min(i + 1000, len(x))
        seg_len = end - i
        seg_t = t(seg_len / SAMPLE_RATE)
        x[i:end] += 0.3 * np.sin(2 * np.pi * 2000 * seg_t) * np.exp(-seg_t * 30)
    x = fade(x, 0.01, 0.1)
    return x
sounds.append(("typing-01.wav", "sfx.typing.01", "Typing 01", "интерфейс", typing(), 0.4, ["typing", "клавиатура"]))

# 9. Camera shutter
def shutter():
    dur = 0.15
    x = 0.6 * noise(dur, 0.8)
    x = fade(x, 0.005, 0.05)
    return x * 0.5
sounds.append(("shutter-01.wav", "sfx.shutter.01", "Camera Shutter", "техника", shutter(), 0.15, ["shutter", "камера", "click"]))

# 10. Notification
def notification():
    dur = 0.4
    tt = t(dur)
    x = 0.4 * (np.sin(2 * np.pi * 1200 * tt) + 0.5 * np.sin(2 * np.pi * 1600 * tt))
    x *= np.exp(-tt * 5)
    x = fade(x, 0.02, 0.2)
    return x
sounds.append(("notification-01.wav", "sfx.notification.01", "Notification", "интерфейс", notification(), 0.4, ["notification", "уведомление"]))

# 11. Sparkle
def sparkle():
    dur = 0.6
    x = np.zeros(int(dur * SAMPLE_RATE))
    seg = 0.3 * np.sin(2 * np.pi * 8000 * t(0.1)) * np.exp(-t(0.1)*50)
    for i in range(len(x)):
        if np.random.random() < 0.002:
            end = min(i + len(seg), len(x))
            x[i:end] += seg[:end-i]
    x = fade(x, 0.01, 0.3)
    return x
sounds.append(("sparkle-01.wav", "sfx.sparkle.01", "Sparkle 01", "переходы", sparkle(), 0.6, ["sparkle", "блеск"]))

# 12. Sub-drop
def sub_drop():
    dur = 0.8
    tt = t(dur)
    freq = 100 * np.exp(-tt * 5)
    phase = 2 * np.pi * np.cumsum(freq) / SAMPLE_RATE
    x = 0.7 * np.sin(phase)
    x = fade(x, 0.05, 0.5)
    return x
sounds.append(("subdrop-01.wav", "sfx.subdrop.01", "Sub Drop 01", "удары", sub_drop(), 0.8, ["sub-drop", "bass"]))

# 13. Buzzer
def buzzer():
    dur = 0.5
    tt = t(dur)
    x = 0.4 * np.sign(np.sin(2 * np.pi * 440 * tt))
    x = fade(x, 0.01, 0.2)
    return x
sounds.append(("buzzer-01.wav", "sfx.buzzer.01", "Buzzer 01", "интерфейс", buzzer(), 0.5, ["buzzer", "сигнал"]))

# 14. Chime
def chime():
    dur = 1.0
    tt = t(dur)
    x = 0.3 * (np.sin(2 * np.pi * 523.25 * tt) + np.sin(2 * np.pi * 659.25 * tt) + np.sin(2 * np.pi * 783.99 * tt))
    x *= np.exp(-tt * 2)
    x = fade(x, 0.01, 0.5)
    return x
sounds.append(("chime-01.wav", "sfx.chime.01", "Chime 01", "интерфейс", chime(), 1.0, ["chime", "звон"]))

# 15. Drum hit
def drum():
    dur = 0.5
    tt = t(dur)
    x = 0.5 * np.sin(2 * np.pi * 100 * tt) * np.exp(-tt * 15)
    x += 0.2 * noise(dur, 0.5) * np.exp(-tt * 40)
    x = fade(x, 0.01, 0.3)
    return x
sounds.append(("drum-01.wav", "sfx.drum.01", "Drum Hit 01", "удары", drum(), 0.5, ["drum", "удар", "percussion"]))

# 16. Laser
def laser():
    dur = 0.4
    tt = t(dur)
    freq = 2000 * np.exp(-tt * 8)
    phase = 2 * np.pi * np.cumsum(freq) / SAMPLE_RATE
    x = 0.4 * np.sin(phase)
    x = fade(x, 0.01, 0.2)
    return x
sounds.append(("laser-01.wav", "sfx.laser.01", "Laser 01", "техника", laser(), 0.4, ["laser", "sci-fi"]))

# 17. Explosion
def explosion():
    dur = 1.0
    x = noise(dur, 0.4)
    for i in range(len(x)):
        alpha = 200 / (200 + SAMPLE_RATE)
        if i > 0:
            x[i] = alpha * x[i] + (1 - alpha) * x[i-1]
    x *= np.exp(-t(dur) * 3)
    x = fade(x, 0.02, 0.5)
    return x
sounds.append(("explosion-01.wav", "sfx.explosion.01", "Explosion 01", "удары", explosion(), 1.0, ["explosion", "взрыв"]))

# 18. Alarm
def alarm():
    dur = 0.8
    tt = t(dur)
    freq = 800 + 400 * np.sin(2 * np.pi * 5 * tt)
    phase = 2 * np.pi * np.cumsum(freq) / SAMPLE_RATE
    x = 0.4 * np.sin(phase)
    x = fade(x, 0.05, 0.2)
    return x
sounds.append(("alarm-01.wav", "sfx.alarm.01", "Alarm 01", "интерфейс", alarm(), 0.8, ["alarm", "тревога"]))

# 19. Water drop
def water_drop():
    dur = 0.4
    tt = t(dur)
    x = 0.3 * np.sin(2 * np.pi * 300 * tt) * np.exp(-tt * 20)
    x += 0.1 * np.sin(2 * np.pi * 600 * tt) * np.exp(-tt * 15)
    x = fade(x, 0.01, 0.2)
    return x
sounds.append(("waterdrop-01.wav", "sfx.waterdrop.01", "Water Drop", "природа", water_drop(), 0.4, ["water", "капля"]))

# 20. Wind gust
def wind():
    dur = 1.0
    x = noise(dur, 0.25)
    for i in range(len(x)):
        cutoff = 100 + 500 * math.sin(i / len(x) * math.pi * 2)
        alpha = cutoff / (cutoff + SAMPLE_RATE)
        if i > 0:
            x[i] = alpha * x[i] + (1 - alpha) * x[i-1]
    x = fade(x, 0.1, 0.3)
    return x
sounds.append(("wind-01.wav", "sfx.wind.01", "Wind Gust", "природа", wind(), 1.0, ["wind", "ветер"]))

# 21. Coin collect
def coin():
    dur = 0.3
    tt = t(dur)
    x = 0.4 * (np.sin(2 * np.pi * 1000 * tt) + np.sin(2 * np.pi * 1500 * tt))
    x *= np.exp(-tt * 10)
    x = fade(x, 0.01, 0.15)
    return x
sounds.append(("coin-01.wav", "sfx.coin.01", "Coin Collect", "интерфейс", coin(), 0.3, ["coin", "сбор"]))

# 22. Page turn
def page_turn():
    dur = 0.3
    x = noise(dur, 0.2)
    for i in range(len(x)):
        alpha = 500 / (500 + SAMPLE_RATE)
        if i > 0:
            x[i] = alpha * x[i] + (1 - alpha) * x[i-1]
    x = fade(x, 0.02, 0.1)
    return x
sounds.append(("page_turn-01.wav", "sfx.page_turn.01", "Page Turn", "интерфейс", page_turn(), 0.3, ["page", "turn"]))

# 23. Error buzz
def error():
    dur = 0.4
    tt = t(dur)
    x = 0.4 * np.sign(np.sin(2 * np.pi * 200 * tt))
    x = fade(x, 0.01, 0.1)
    return x
sounds.append(("error-01.wav", "sfx.error.01", "Error Buzz", "интерфейс", error(), 0.4, ["error", "ошибка"]))

# 24. Success chime
def success():
    dur = 0.6
    tt = t(dur)
    x = 0.3 * (np.sin(2 * np.pi * 523.25 * tt) + np.sin(2 * np.pi * 783.99 * tt) + np.sin(2 * np.pi * 1046.50 * tt))
    x *= np.exp(-tt * 3)
    x = fade(x, 0.01, 0.3)
    return x
sounds.append(("success-01.wav", "sfx.success.01", "Success Chime", "интерфейс", success(), 0.6, ["success", "успех"]))

# 25. Ambient drone (music loop)
def drone():
    dur = 4.0
    tt = t(dur)
    x = 0.05 * (np.sin(2 * np.pi * 55 * tt) + np.sin(2 * np.pi * 82.41 * tt) + np.sin(2 * np.pi * 110 * tt))
    x = fade(x, 1.0, 1.0)
    return x
sounds.append(("drone-01.wav", "music.drone.01", "Ambient Drone", "музыка", drone(), 4.0, ["ambient", "loop", "muzyka"]))

# Generate all files
catalog_items = []
for fname, sid, title, category, samples, duration, tags in sounds:
    write_wav(os.path.join(OUT, fname), samples)
    print(f"Generated {fname} ({duration:.1f}s)")
    catalog_items.append({
        "id": sid,
        "kind": "sfx" if sid.startswith("sfx") else "music",
        "title": title,
        "category": category,
        "path": fname,
        "duration": round(duration, 2),
        "tags": tags,
    })

catalog = {"version": 1, "items": catalog_items}
import json
with open(os.path.join(OUT, "catalog.json"), "w", encoding="utf-8") as f:
    json.dump(catalog, f, ensure_ascii=False, indent=2)

print(f"\nGenerated {len(sounds)} sounds + catalog.json")

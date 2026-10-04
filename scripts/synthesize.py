"""Generate original procedural audio. No sampled or third-party sound assets."""
import math, random, struct, wave
from pathlib import Path
out=Path(__file__).resolve().parent.parent/'assets'
rate=22050
random.seed(19)
def write(name, samples):
    with wave.open(str(out/(name+'.wav')), 'wb') as w:
        w.setparams((1,2,rate,0,'NONE','not compressed'))
        w.writeframes(b''.join(struct.pack('<h',int(max(-1,min(1,x))*27000)) for x in samples))
for name, duration, freq, noise in [('slash',.16,640,.5),('hit',.18,100,.5),('jump',.18,460,.05),('dodge',.2,140,.35),('parry',.45,1100,.08),('loot',.5,660,0),('hurt',.3,75,.5),('explosion',.65,50,.8),('heal',.8,440,.02)]:
    samples=[]
    for i in range(int(rate*duration)):
        t=i/rate;u=t/duration
        pitch=freq*(1+u if name in ['jump','loot','heal'] else 1-u*.65)
        value=(math.sin(2*math.pi*pitch*t)*(1-noise)+random.uniform(-1,1)*noise)*(1-u)**2*.45
        samples.append(value)
    write(name,samples)
# Seamless 16-second ambient bed: D minor, bell-like plucks and a soft pulse.
duration=16
samples=[]
notes=[146.832,174.614,220.,261.626,220.,174.614,130.813,164.814]
for i in range(rate*duration):
    t=i/rate
    pad=sum(math.sin(2*math.pi*f*t) for f in [55,82.5,110,165])/4*.075
    phase=t%2; note=notes[int(t/2)]
    bell=(math.sin(2*math.pi*note*2*phase)+.3*math.sin(2*math.pi*note*4*phase))*math.exp(-phase*3)*.06
    pulse=math.sin(2*math.pi*55*(t%.5))*math.exp(-(t%.5)*30)*.08
    fade=min(1,t*2,(duration-t)*2)
    samples.append((pad+bell+pulse)*fade)
write('ambience',samples)

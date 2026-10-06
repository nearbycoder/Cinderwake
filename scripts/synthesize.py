"""Generate original procedural audio. No sampled or third-party sound assets.

Run from anywhere: `python3 scripts/synthesize.py` rewrites every file under
assets/ and checks the music loops. `--check FILE...` checks existing loops only.
"""
import math, random, struct, sys, wave
from pathlib import Path
out=Path(__file__).resolve().parent.parent/'assets'
rate=22050
random.seed(19)
def write(name, samples, scale=27000):
    path=out/(name+'.wav')
    path.parent.mkdir(exist_ok=True)
    with wave.open(str(path), 'wb') as w:
        w.setparams((1,2,rate,0,'NONE','not compressed'))
        w.writeframes(b''.join(struct.pack('<h',int(max(-1,min(1,x))*scale)) for x in samples))

def effects():
    for name, duration, freq, noise in [('slash',.16,640,.5),('hit',.18,100,.5),('jump',.18,460,.05),('dodge',.2,140,.35),('parry',.45,1100,.08),('loot',.5,660,0),('hurt',.3,75,.5),('explosion',.65,50,.8),('heal',.8,440,.02)]:
        samples=[]
        for i in range(int(rate*duration)):
            t=i/rate;u=t/duration
            pitch=freq*(1+u if name in ['jump','loot','heal'] else 1-u*.65)
            value=(math.sin(2*math.pi*pitch*t)*(1-noise)+random.uniform(-1,1)*noise)*(1-u)**2*.45
            samples.append(value)
        write(name,samples)

# ---------------------------------------------------------------------------
# Music. Each track is a whole number of bars. Notes are rendered into a ring
# buffer, so any tail that runs past the end rings into the start, and the
# loop wraps with no fade or gap. Sustained drones use frequencies with a whole
# number of cycles per loop, and filtered noise runs twice around the ring so
# its filter state matches at the seam.
TAU=2*math.pi
NOTE={'C':-9,'C#':-8,'Db':-8,'D':-7,'D#':-6,'Eb':-6,'E':-5,'F':-4,'F#':-3,'Gb':-3,'G':-2,'G#':-1,'Ab':-1,'A':0,'A#':1,'Bb':1,'B':2}
def hz(name):
    """'A4' -> 440 Hz."""
    return 440*2**((NOTE[name[:-1]]+12*(int(name[-1])-4))/12)

class Track:
    def __init__(self, bpm, beats, seed):
        self.beat=60/bpm
        self.n=round(beats*self.beat*rate)
        self.length=self.n/rate
        self.buf=[0.]*self.n
        self.rng=random.Random(seed)
    def at(self, beat):
        return round(beat*self.beat*rate)
    def add(self, start, samples):
        n=self.n; buf=self.buf
        for i,v in enumerate(samples):
            buf[(start+i)%n]+=v
    def loop_hz(self, f):
        return round(f*self.length)/self.length
    def note(self, beat, freq, dur, amp, kind):
        self.add(self.at(beat), voice(kind, freq, dur*self.beat, amp, self.rng))
    def drone(self, freq, amp, wobble=0.):
        f=self.loop_hz(freq); lfo=1/self.length*2
        for i in range(self.n):
            t=i/rate
            self.buf[i]+=amp*(1+wobble*math.sin(TAU*lfo*t))*(math.sin(TAU*f*t)+.3*math.sin(TAU*2*f*t))
    def wind(self, amp, smooth, swells):
        """Low-passed noise whose loudness swells `swells` times per loop."""
        noise=[self.rng.uniform(-1,1) for _ in range(self.n)]
        y=0.; out=[0.]*self.n
        for lap in range(2):
            for i,v in enumerate(noise):
                y+=smooth*(v-y)
                out[i]=y
        for i in range(self.n):
            t=i/self.n
            self.buf[i]+=out[i]*amp*(.55+.45*math.sin(TAU*swells*t))
    def finish(self, name, rms_db=-31., peak=.6):
        rms=math.sqrt(sum(v*v for v in self.buf)/self.n)
        gain=10**(rms_db/20)/rms
        top=max(abs(v) for v in self.buf)*gain
        if top>peak:
            gain*=peak/top
        write('music/'+name,[v*gain for v in self.buf],scale=32767)

def voice(kind, freq, dur, amp, rng):
    """One note: an attack, a body, and a release that may outlast `dur`."""
    out=[]
    if kind=='pad':
        # Slow swell, two detuned voices, soft harmonics.
        attack=min(1.2,dur*.4); total=dur+1.4
        for i in range(int(total*rate)):
            t=i/rate
            env=min(1,t/attack)*(1 if t<dur else math.exp(-(t-dur)*3.2))
            s=0.
            for d in (-.35,.35):
                p=TAU*(freq+d)*t
                s+=math.sin(p)+.35*math.sin(2*p)+.12*math.sin(3*p)
            out.append(amp*env*s*.5)
    elif kind=='organ':
        attack=.25; total=dur+.8
        for i in range(int(total*rate)):
            t=i/rate
            env=min(1,t/attack)*(1 if t<dur else math.exp(-(t-dur)*5))
            p=TAU*freq*t
            s=math.sin(p)+.5*math.sin(2*p)+.28*math.sin(3*p)+.16*math.sin(4*p)+.08*math.sin(6*p)
            out.append(amp*env*s*(1+.04*math.sin(TAU*5.2*t)))
    elif kind=='bell':
        total=max(dur,2.4)
        for i in range(int(total*rate)):
            t=i/rate
            s=(math.sin(TAU*freq*t)*math.exp(-t*1.6)
               +.5*math.sin(TAU*freq*2.01*t)*math.exp(-t*2.6)
               +.25*math.sin(TAU*freq*3.03*t)*math.exp(-t*4)
               +.12*math.sin(TAU*freq*4.2*t)*math.exp(-t*6))
            out.append(amp*s*min(1,t*400))
    elif kind=='glass':
        total=max(dur,3.)
        for i in range(int(total*rate)):
            t=i/rate
            s=(math.sin(TAU*freq*t)*math.exp(-t*1.1)
               +.4*math.sin(TAU*freq*2.76*t)*math.exp(-t*2.2)
               +.2*math.sin(TAU*freq*5.4*t)*math.exp(-t*4.5))
            out.append(amp*s*min(1,t*300))
    elif kind=='pluck':
        total=dur+.6
        for i in range(int(total*rate)):
            t=i/rate
            p=TAU*freq*t
            out.append(amp*(math.sin(p)+.45*math.sin(2*p)*math.exp(-t*9)+.2*math.sin(3*p)*math.exp(-t*14))*math.exp(-t*4.2)*min(1,t*500))
    elif kind=='drip':
        # A water drop: a fast upward chirp with a short ring.
        total=.35; ph=0.
        for i in range(int(total*rate)):
            t=i/rate
            ph+=TAU*freq*(1+1.2*min(1,t/.04))/rate
            out.append(amp*math.sin(ph)*math.exp(-t*16)*min(1,t*800))
    elif kind=='thump':
        total=.9; ph=0.
        for i in range(int(total*rate)):
            t=i/rate
            ph+=TAU*freq*(1+1.5*math.exp(-t*25))/rate
            out.append(amp*math.sin(ph)*math.exp(-t*5)*min(1,t*600))
    elif kind=='anvil':
        partials=[(1,1),(2.42,.6),(3.87,.4),(5.11,.3),(6.9,.2)]
        total=1.6
        for i in range(int(total*rate)):
            t=i/rate
            s=sum(a*math.sin(TAU*freq*r*t)*math.exp(-t*(3+r*1.4)) for r,a in partials)
            s+=rng.uniform(-1,1)*math.exp(-t*60)*1.2
            out.append(amp*s*.5)
    elif kind=='tick':
        total=.06
        for i in range(int(total*rate)):
            t=i/rate
            out.append(amp*(math.sin(TAU*freq*t)+rng.uniform(-.6,.6))*math.exp(-t*90))
    elif kind=='crackle':
        total=.02
        for i in range(int(total*rate)):
            t=i/rate
            out.append(amp*rng.uniform(-1,1)*math.exp(-t*300))
    else:
        raise ValueError(kind)
    return out

def chords(track, progression, beats_each, amp, kind='pad'):
    for bar,chord in enumerate(progression):
        for name in chord:
            track.note(bar*beats_each, hz(name), beats_each, amp, kind)

def hearth():
    """Title, Keeper, and rest screens: D minor, 60 BPM, a fire's crackle."""
    t=Track(60,20,101)
    prog=[['D3','F3','A3'],['Bb2','D3','F3'],['F2','A2','C3'],['C3','E3','G3'],['D3','F3','A3']]
    chords(t,prog,4,.10)
    t.drone(hz('D2'),.05,.3)
    arps=[['D5','A4','F5','A4'],['D5','Bb4','F5','D5'],['C5','A4','F5','C5'],['E5','G4','C5','G4'],['F5','D5','A4','D5']]
    for bar,notes in enumerate(arps):
        for j,name in enumerate(notes):
            if (bar+j)%5!=4:
                t.note(bar*4+j+.5*(j%2),hz(name),1,.05,'bell')
    for _ in range(70):
        t.note(t.rng.uniform(0,20),0,0,t.rng.uniform(.04,.12),'crackle')
    t.finish('hearth')

def aqueduct():
    """The Drowned Aqueduct: D dorian, 75 BPM, drips and a distant pump."""
    t=Track(75,24,202)
    prog=[['D3','F3','A3','C4'],['E3','G3','B3','D4'],['C3','E3','G3','B3']]
    chords(t,prog,8,.08)
    t.drone(hz('D2'),.045,.4)
    t.wind(.18,.004,3)
    for beat in range(0,24,2):
        t.note(beat,hz('D2'),.5,.18 if beat%4==0 else .1,'thump')
    scale=['D5','E5','F5','G5','A5','B5','C6','D6']
    for _ in range(26):
        t.note(t.rng.uniform(0,24),hz(t.rng.choice(scale)),.2,.07,'drip')
    melody=[(1,'A4'),(3,'C5'),(4.5,'D5'),(9,'B4'),(11,'G4'),(12.5,'E4'),(17,'G4'),(19,'E4'),(20.5,'D4')]
    for beat,name in melody:
        t.note(beat,hz(name),1.5,.06,'pluck')
    t.finish('aqueduct')

def conservatory():
    """Glassroot Conservatory: F lydian, 90 BPM, glass chimes and wind."""
    t=Track(90,32,303)
    prog=[['F3','A3','C4','E4'],['G3','B3','D4','F4'],['E3','G3','B3','D4'],['A2','C3','E3','G3']]
    chords(t,prog,8,.07)
    t.drone(hz('F2'),.035,.3)
    t.wind(.22,.01,2)
    arps=[['F5','A5','C6','E6','B5','C6'],['G5','B5','D6','F6','D6','B5'],['E5','G5','B5','D6','B5','G5'],['A5','C6','E6','G6','E6','C6']]
    for bar,notes in enumerate(arps):
        for j,name in enumerate(notes):
            t.note(bar*8+j*1.25+(.5 if j%3==2 else 0),hz(name),1,.035,'glass')
    for _ in range(14):
        t.note(t.rng.uniform(0,32),hz(t.rng.choice(['C7','E7','G7','B6'])),.5,.012,'glass')
    t.finish('conservatory')

def foundry():
    """The Ember Foundry: C phrygian, 100 BPM, anvils and bellows."""
    t=Track(100,32,404)
    prog=[['C3','Eb3','G3'],['Db3','F3','Ab3'],['C3','Eb3','G3'],['Bb2','D3','F3']]
    chords(t,prog,8,.07)
    t.drone(hz('C2'),.07,.25)
    t.drone(hz('G2'),.03,.25)
    t.wind(.3,.003,8)
    ostinato=['C2','C2','Db2','C2','C2','C2','Bb1','C2']
    for beat in range(32):
        t.note(beat,hz(ostinato[beat%8]),.6,.09,'pluck')
    for bar in range(8):
        t.note(bar*4,hz('C2'),.5,.2,'thump')
        t.note(bar*4+1.5,hz('C5'),.3,.06,'anvil')
        if bar%2==1:
            t.note(bar*4+3,hz('G4'),.3,.045,'anvil')
    t.finish('foundry')

def crown():
    """Crown of the Machine: A harmonic minor, 84 BPM, an organ and a clock."""
    t=Track(84,28,505)
    prog=[['A2','C3','E3','A3'],['F2','A2','C3','F3'],['D3','F3','A3','D4'],['E2','G#2','B2','E3']]
    for bar,chord in enumerate(prog):
        beats=8 if bar<3 else 4
        start=[0,8,16,24][bar]
        for name in chord:
            t.note(start,hz(name),beats,.045,'organ')
    t.drone(hz('A1'),.06,.2)
    for beat in range(28):
        t.note(beat,2800 if beat%2==0 else 2100,.05,.05,'tick')
    for start in (0,8,16,24):
        t.note(start,hz('A1'),.5,.22,'thump')
    melody=[(2,'E5'),(3,'F5'),(4,'E5'),(6,'C5'),(10,'C5'),(11,'D5'),(12,'C5'),(14,'A4'),(18,'D5'),(19,'F5'),(20,'A5'),(22,'G#5'),(25,'B4'),(26,'G#4')]
    for beat,name in melody:
        t.note(beat,hz(name),1,.045,'bell')
    t.finish('crown')

MUSIC=['hearth','aqueduct','conservatory','foundry','crown']

def read(path):
    with wave.open(str(path)) as w:
        n=w.getnframes()
        return [v/32768 for v in struct.unpack('<%dh'%n, w.readframes(n))], w.getframerate()

def check(path, tolerance_db=2.):
    """A seamless loop doesn't clip, doesn't jump at the seam, and keeps its
    loudness through the seam. Returns a list of problems."""
    x,r=read(path)
    db=lambda a: 20*math.log10(max(math.sqrt(sum(v*v for v in a)/len(a)),1e-9))
    whole=db(x)
    w=r//4
    seam=x[-w:]+x[:w]
    steps=sorted(abs(x[i+1]-x[i]) for i in range(len(x)-1))
    p999=steps[int(len(steps)*.999)]
    jump=abs(x[0]-x[-1])
    peak=max(abs(v) for v in x)
    report=f'{path.name}: {len(x)/r:.1f} s, peak {peak:.2f}, RMS {whole:.1f} dBFS, seam ±0.25 s {db(seam)-whole:+.1f} dB, seam step {jump:.4f} (99.9% of steps <= {p999:.4f})'
    problems=[]
    if peak>=.99: problems.append('clips')
    if jump>p999: problems.append('jumps at the seam')
    if abs(db(seam)-whole)>tolerance_db: problems.append(f'seam loudness differs by more than {tolerance_db} dB')
    print(report+('  FAIL: '+', '.join(problems) if problems else '  ok'))
    return problems

if __name__=='__main__':
    if sys.argv[1:2]==['--check']:
        sys.exit(1 if any([check(Path(p)) for p in sys.argv[2:]]) else 0)
    effects()
    for make in (hearth,aqueduct,conservatory,foundry,crown):
        make()
    if any([check(out/'music'/(name+'.wav')) for name in MUSIC]):
        sys.exit(1)

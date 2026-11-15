const verseText = [
  'The preparations of the heart in man, and the answer of the tongue, is from the LORD.',
  'All the ways of a man are clean in his own eyes; but the LORD weigheth the spirits.',
  'Commit thy works unto the LORD, and thy thoughts shall be established.',
  'The LORD hath made all things for himself: yea, even the wicked for the day of evil.',
  'Every one that is proud in heart is an abomination to the LORD: though hand join in hand, he shall not be unpunished.',
];
const studies = [...document.querySelectorAll('.study')];
const destination = document.querySelector('#destination');
const status = document.querySelector('#playback-status');
const timers = new WeakMap();
const cueDuration = 3000;

for (const study of studies) {
  const reader = study.querySelector('.reader');
  const title = document.createElement('h3');
  title.className = 'chapter-title';
  title.textContent = 'Proverbs';
  const meta = document.createElement('p');
  meta.className = 'chapter-meta';
  meta.textContent = 'Chapter 16 · King James Version';
  reader.append(title, meta);
  verseText.forEach((text, index) => {
    const verse = document.createElement('div');
    verse.className = 'verse';
    verse.dataset.verse = String(index + 1);
    const number = document.createElement('span');
    number.className = 'verse-number';
    number.textContent = String(index + 1);
    const paragraph = document.createElement('p');
    paragraph.className = 'verse-text';
    const start = document.createElement('span');
    start.className = 'verse-start';
    const words = text.split(' ');
    start.textContent = words.slice(0, 3).join(' ');
    paragraph.append(start, document.createTextNode(' ' + words.slice(3).join(' ')));
    verse.append(number, paragraph);
    reader.append(verse);
  });
  study.querySelector('.replay').addEventListener('click', () => {
    play(study);
    status.textContent = `${study.querySelector('h2').textContent} · Proverbs 16:${destination.value}`;
  });
}

function position(study, target) {
  const reader = study.querySelector('.reader');
  // Land first, then run the cue. No simultaneous scrolling and cue motion.
  reader.scrollTop += target.getBoundingClientRect().top - reader.getBoundingClientRect().top - reader.clientHeight * .26;
}

function play(study) {
  clearTimeout(timers.get(study));
  delete study.dataset.playing;
  study.querySelectorAll('.target').forEach(verse => verse.classList.remove('target'));
  const target = study.querySelector(`[data-verse="${destination.value}"]`);
  target.classList.add('target');
  study.querySelector('.phone-reference').textContent = `Proverbs 16:${destination.value}`;
  position(study, target);
  void study.offsetWidth;
  study.dataset.playing = 'true';
  timers.set(study, setTimeout(() => {
    delete study.dataset.playing;
    target.classList.remove('target');
  }, cueDuration));
}

document.querySelector('#navigation').addEventListener('submit', event => {
  event.preventDefault();
  studies.forEach(play);
  status.textContent = `Playing all three · Proverbs 16:${destination.value}`;
});
document.querySelector('#theme').addEventListener('click', event => {
  const dark = document.documentElement.dataset.theme !== 'dark';
  document.documentElement.dataset.theme = dark ? 'dark' : 'light';
  event.currentTarget.setAttribute('aria-pressed', String(dark));
});
const systemMotion = matchMedia('(prefers-reduced-motion: reduce)');
function syncMotion() {
  document.querySelector('#system-motion').hidden = !systemMotion.matches;
}
syncMotion();
systemMotion.addEventListener('change', syncMotion);
document.querySelector('#reduced').addEventListener('change', event => {
  document.body.classList.toggle('reduced', event.currentTarget.checked);
});

// All three begin at the same reading location, without starting a cue.
document.fonts.ready.then(() => {
  studies.forEach(study => position(study, study.querySelector('[data-verse="3"]')));
});

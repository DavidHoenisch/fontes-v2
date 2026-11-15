const text = [
  'The preparations of the heart in man, and the answer of the tongue, is from the LORD.',
  'All the ways of a man are clean in his own eyes; but the LORD weigheth the spirits.',
  'Commit thy works unto the LORD, and thy thoughts shall be established.',
  'The LORD hath made all things for himself: yea, even the wicked for the day of evil.',
  'Every one that is proud in heart is an abomination to the LORD: though hand join in hand, he shall not be unpunished.',
];
const phone = document.querySelector('#phone');
const reader = document.querySelector('#reader');
const destination = document.querySelector('#verse');
const playback = document.querySelector('#playback');
const slow = document.querySelector('#slow');
const reduced = document.querySelector('#reduced');
const systemMotion = matchMedia('(prefers-reduced-motion: reduce)');
const effects = {
  combined: {name: 'Settle + resolve', duration: 1600},
  settle: {name: 'Settle', duration: 620},
  resolve: {name: 'Resolve', duration: 780},
};
let effect = 'combined';
let timer;

text.forEach((quote, index) => {
  const verse = document.createElement('div');
  verse.className = 'verse';
  verse.dataset.verse = String(index + 1);
  const number = document.createElement('span');
  number.className = 'verse-number';
  number.textContent = String(index + 1);
  const paragraph = document.createElement('p');
  paragraph.className = 'verse-text';
  paragraph.textContent = quote;
  verse.append(number, paragraph);
  document.querySelector('#verses').append(verse);
});

function clear() {
  clearTimeout(timer);
  phone.classList.remove('playing');
  phone.querySelectorAll('.target').forEach(verse => verse.classList.remove('target'));
}
function position() {
  const target = phone.querySelector(`[data-verse="${destination.value}"]`);
  reader.scrollTop += target.getBoundingClientRect().top - reader.getBoundingClientRect().top - reader.clientHeight * .34;
  return target;
}
function play() {
  clear();
  const target = position();
  if (matchMedia('(max-width: 780px)').matches) {
    phone.scrollIntoView({block: 'start', behavior: 'instant'});
  }
  const reduce = reduced.checked || systemMotion.matches;
  const multiplier = !reduce && slow.checked ? 4 : 1;
  const duration = (reduce ? 1600 : effects[effect].duration) * multiplier;
  phone.dataset.effect = effect;
  phone.style.setProperty('--duration', `${duration}ms`);
  document.querySelector('#reference').textContent = `Proverbs 16:${destination.value}`;
  target.classList.add('target');
  void phone.offsetWidth;
  phone.classList.add('playing');
  playback.textContent = `${effects[effect].name} · ${reduce ? 'static reduced-motion cue' : slow.checked ? '¼ speed' : 'normal speed'} · Proverbs 16:${destination.value}`;
  timer = setTimeout(() => {
    clear();
    playback.textContent = `${effects[effect].name} · at rest`;
  }, duration + 50);
}

document.querySelectorAll('[data-choice]').forEach(button => {
  button.addEventListener('click', () => {
    effect = button.dataset.choice;
    document.querySelectorAll('[data-choice]').forEach(choice => choice.setAttribute('aria-pressed', String(choice === button)));
    play();
  });
});
document.querySelector('#preview-form').addEventListener('submit', event => { event.preventDefault(); play(); });
document.querySelector('#dark').addEventListener('change', event => {
  document.documentElement.dataset.theme = event.currentTarget.checked ? 'dark' : 'light';
});
function syncMotion() {
  const reduce = reduced.checked || systemMotion.matches;
  document.body.classList.toggle('reduced', reduce);
  slow.disabled = reduce;
  if (phone.classList.contains('playing')) play();
}
reduced.addEventListener('change', syncMotion);
systemMotion.addEventListener('change', syncMotion);
syncMotion();
document.fonts.ready.then(position);

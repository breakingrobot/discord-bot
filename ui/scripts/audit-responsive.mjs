// Audit du responsive sur la maquette de l'API : pour chaque page et chaque largeur, mesure le défilement
// horizontal de la page, les éléments qui sortent de l'écran et (écrans tactiles) les cibles de moins de 44 px.
//
//   VITE_MOCK=1 npx vite build --outDir dist-mock && npx vite preview --outDir dist-mock --port 4175
//   node scripts/audit-responsive.mjs
//
// Playwright n'est pas une dépendance du projet : `npm i -g playwright` (ou PLAYWRIGHT_MODULE=/chemin/vers/playwright/index.mjs).
// Variables : BASE (http://localhost:4175), WIDTHS (320,360,390,768,1024,1440), CHROME (chemin de Chromium).
// Code de sortie 1 si une page défile horizontalement ou si un élément sort de l'écran.

/* oxlint-disable no-await-in-loop -- les pages sont parcourues l'une après l'autre, dans un même navigateur. */

const BASE = process.env.BASE ?? 'http://localhost:4175'
const GUILD = '1187000000000000042'
const WIDTHS = (process.env.WIDTHS ?? '320,360,390,768,1024,1440').split(',').map(Number)
const PAGES = [
  '/',
  ...[
    '',
    '/stats',
    '/transactions',
    '/cotisations',
    '/coffres',
    '/contacts',
    '/journal',
    '/membres',
    '/presences',
    '/absences',
    '/recrues',
    '/annuaire',
    '/permissions',
    '/configuration',
    '/sante',
  ].map((page) => `/serveurs/${GUILD}${page}`),
  '/tickets',
  '/tickets/staff',
]

const { chromium } = await import(process.env.PLAYWRIGHT_MODULE ?? 'playwright')
const browser = await chromium.launch(process.env.CHROME ? { executablePath: process.env.CHROME } : {})

/** Exécuté dans la page : éléments hors écran (hors conteneurs défilants) et petites cibles tactiles. */
const measure = (viewport) => {
  const root = document.documentElement
  const outside = []
  const seen = new Set()
  // oxlint-disable-next-line unicorn/consistent-function-scoping -- la fonction est sérialisée avec `measure`.
  const clipped = (element) => {
    for (let parent = element.parentElement; parent && parent !== document.body; parent = parent.parentElement) {
      const { overflowX } = getComputedStyle(parent)
      if (
        /hidden|clip/u.test(overflowX) ||
        (/auto|scroll/u.test(overflowX) && parent.scrollWidth > parent.clientWidth + 1)
      ) {
        return true
      }
    }
    return false
  }
  for (const element of document.querySelectorAll('body *')) {
    const style = getComputedStyle(element)
    if (style.display === 'none' || style.visibility === 'hidden' || style.position === 'fixed') continue
    const box = element.getBoundingClientRect()
    if (box.width === 0 || box.height === 0 || (box.right <= viewport + 1 && box.left >= -1) || clipped(element)) {
      continue
    }
    const key = `${element.tagName}.${String(element.className).slice(0, 60)}`
    if (seen.has(key)) continue
    seen.add(key)
    outside.push(
      `${key} [${Math.round(box.left)}..${Math.round(box.right)}] « ${(element.textContent ?? '').trim().slice(0, 30)} »`,
    )
  }
  const small = []
  if (matchMedia('(pointer: coarse)').matches) {
    const targets = 'a, button, [role=button], [role=tab], input, select, textarea'
    for (const element of document.querySelectorAll(targets)) {
      const box = element.getBoundingClientRect()
      const style = getComputedStyle(element)
      const hidden = style.display === 'none' || style.visibility === 'hidden' || box.width <= 2
      // Les zones tactiles élargies (`tap`, cases, interrupteurs) dépassent l'élément lui-même.
      const widened = element.matches('.tap, [data-slot=checkbox], [data-slot=switch]') || element.closest('.sr-only')
      const inline = element.tagName === 'A' && style.display === 'inline'
      if (hidden || widened || inline || element.getAttribute('aria-hidden') === 'true' || box.height >= 43.5) continue
      small.push(
        `${element.tagName}:${(element.getAttribute('aria-label') ?? element.textContent ?? '').trim().slice(0, 25)} ${Math.round(box.width)}x${Math.round(box.height)}`,
      )
    }
  }
  return {
    scroll: root.scrollWidth - root.clientWidth,
    outside: outside.slice(0, 6),
    small: [...new Set(small)].slice(0, 8),
  }
}

let failures = 0
for (const width of WIDTHS) {
  const touch = width < 800
  const context = await browser.newContext({ viewport: { width, height: 800 }, hasTouch: touch, isMobile: touch })
  const page = await context.newPage()
  await page.goto(`${BASE}/?mock=admin`)
  for (const path of PAGES) {
    await page.goto(`${BASE}${path}`, { waitUntil: 'networkidle' })
    await page.waitForTimeout(400)
    const result = await page.evaluate(measure, width)
    const broken = result.scroll > 0 || result.outside.length > 0
    if (broken) failures += 1
    if (broken || result.small.length > 0) {
      console.log(`${broken ? '✗' : '·'} ${width}px ${path.replace(GUILD, ':guild')}  défilement=${result.scroll}`)
      for (const line of result.outside) console.log(`    hors écran : ${line}`)
      for (const line of result.small) console.log(`    cible < 44 px : ${line}`)
    }
  }
  await context.close()
}
await browser.close()
console.log(failures === 0 ? 'Aucun débordement.' : `${failures} page(s) avec débordement.`)
process.exit(failures === 0 ? 0 : 1)

document.getElementById('export').addEventListener('click', async () => {
  const status = document.getElementById('status');
  const button = document.getElementById('export');

  const [tab] = await chrome.tabs.query({ active: true, currentWindow: true });

  if (!tab.url?.includes('read.amazon.com')) {
    status.textContent = 'Please navigate to read.amazon.com/kindle-library first.';
    return;
  }

  button.disabled = true;
  status.innerHTML = 'Scrolling through library...';

  try {
    const results = await chrome.scripting.executeScript({
      target: { tabId: tab.id },
      func: scrapeLibrary
    });

    const books = results[0].result;

    if (!books || books.length === 0) {
      status.textContent = 'No books found. Make sure you are on the library page.';
      button.disabled = false;
      return;
    }

    const blob = new Blob([JSON.stringify(books, null, 2)], { type: 'application/json' });
    const url = URL.createObjectURL(blob);
    const filename = `kindle-library-${new Date().toISOString().split('T')[0]}.json`;

    await chrome.downloads.download({ url, filename });

    status.innerHTML = `Exported <span class="count">${books.length}</span> books to ${filename}`;
  } catch (err) {
    status.textContent = 'Error: ' + err.message;
  }

  button.disabled = false;
});

async function scrapeLibrary() {
  const seenAsins = new Set();
  const books = [];

  // First, collect any books from the embedded JSON (initial load)
  const initialData = document.getElementById('itemViewResponse');
  if (initialData) {
    try {
      const data = JSON.parse(initialData.textContent);
      for (const item of data.itemsList || []) {
        if (!seenAsins.has(item.asin)) {
          seenAsins.add(item.asin);
          books.push({
            asin: item.asin,
            title: item.title,
            authors: (item.authors || []).map(a => a.replace(/:$/, '').trim()),
            coverUrl: item.productUrl,
            percentageRead: item.percentageRead,
            resourceType: item.resourceType,
            originType: item.originType
          });
        }
      }
    } catch (e) {
      console.log('Could not parse initial JSON:', e);
    }
  }

  // Find the scrollable container - it's not the document body
  function findScrollContainer() {
    // Try common selectors for the Kindle library
    const candidates = [
      document.querySelector('[class*="library"]'),
      document.querySelector('[class*="content"]'),
      document.querySelector('#web-library-root > div > div'),
      document.querySelector('[class*="scroll"]'),
    ];

    for (const el of candidates) {
      if (el && el.scrollHeight > el.clientHeight) {
        return el;
      }
    }

    // Fallback: find any element with overflow scroll/auto that has scrollable content
    const all = document.querySelectorAll('*');
    for (const el of all) {
      const style = getComputedStyle(el);
      const isScrollable = style.overflowY === 'scroll' || style.overflowY === 'auto';
      if (isScrollable && el.scrollHeight > el.clientHeight + 100) {
        return el;
      }
    }

    return document.scrollingElement || document.body;
  }

  const scrollContainer = findScrollContainer();
  console.log('Using scroll container:', scrollContainer);

  let stableRounds = 0;
  const maxStableRounds = 5;
  const scrollDelay = 1000;

  while (stableRounds < maxStableRounds) {
    const prevCount = seenAsins.size;

    // Scroll to bottom
    scrollContainer.scrollTop = scrollContainer.scrollHeight;
    window.scrollTo(0, document.body.scrollHeight); // Also try window scroll

    // Wait for content to load
    await new Promise(r => setTimeout(r, scrollDelay));

    // Collect books from DOM
    const coverContainers = document.querySelectorAll('[id^="coverContainer-"]');
    for (const container of coverContainers) {
      const asin = container.id.replace('coverContainer-', '');
      if (seenAsins.has(asin)) continue;

      seenAsins.add(asin);

      const titleEl = document.querySelector(`#title-${asin} p`);
      const authorEl = document.querySelector(`#author-${asin} p`);
      const coverEl = document.getElementById(`cover-${asin}`);

      books.push({
        asin,
        title: titleEl?.textContent?.trim() || '',
        authors: authorEl?.textContent?.trim().replace(/:$/, '').split(/,\s*/) || [],
        coverUrl: coverEl?.src || '',
        percentageRead: null,
        resourceType: 'EBOOK',
        originType: 'PURCHASE'
      });
    }

    if (seenAsins.size === prevCount) {
      stableRounds++;
    } else {
      stableRounds = 0;
    }
  }

  return books;
}

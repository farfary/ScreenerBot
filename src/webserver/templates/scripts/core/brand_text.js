// Apply the product typeface to the ScreenerBot name wherever visible copy is rendered.
(function () {
  "use strict";

  const BRAND_NAME = "ScreenerBot";
  const BRAND_CLASS = "brand-name";
  const SKIP_SELECTOR = `.${BRAND_CLASS}, script, style, textarea, option, svg`;

  function shouldStyle(textNode) {
    if (textNode.nodeType !== Node.TEXT_NODE || !textNode.data.includes(BRAND_NAME)) return false;
    const parent = textNode.parentElement;
    return Boolean(parent && !parent.closest(SKIP_SELECTOR));
  }

  function styleTextNode(textNode) {
    if (!shouldStyle(textNode)) return;

    const parts = textNode.data.split(BRAND_NAME);
    const fragment = document.createDocumentFragment();

    parts.forEach((part, index) => {
      if (part) fragment.append(document.createTextNode(part));
      if (index === parts.length - 1) return;

      const brand = document.createElement("span");
      brand.className = BRAND_CLASS;
      brand.textContent = BRAND_NAME;
      fragment.append(brand);
    });

    textNode.replaceWith(fragment);
  }

  function styleTree(root) {
    if (root.nodeType === Node.TEXT_NODE) {
      styleTextNode(root);
      return;
    }

    if (root.nodeType !== Node.ELEMENT_NODE || root.matches(SKIP_SELECTOR)) return;
    if (!root.textContent.includes(BRAND_NAME)) return;

    const matches = [];
    const walker = document.createTreeWalker(root, window.NodeFilter.SHOW_TEXT, {
      acceptNode: (node) =>
        shouldStyle(node)
          ? window.NodeFilter.FILTER_ACCEPT
          : window.NodeFilter.FILTER_REJECT,
    });

    while (walker.nextNode()) matches.push(walker.currentNode);
    matches.forEach(styleTextNode);
  }

  function styleMutations(records) {
    records.forEach((record) => {
      if (record.type === "characterData") {
        styleTextNode(record.target);
        return;
      }

      record.addedNodes.forEach(styleTree);
    });
  }

  styleTree(document.body);

  const observer = new MutationObserver(styleMutations);
  observer.observe(document.body, {
    childList: true,
    subtree: true,
    characterData: true,
  });
})();

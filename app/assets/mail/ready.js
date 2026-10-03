// Whether the mailbox page has finished loading (plan T9.2): the runner's own probe, for any
// provider, run before a step on a page that just navigated. It reads no mail and returns one word,
// `document.readyState`. Like every step script, it is the body of an async function given `args`.
return document.readyState;

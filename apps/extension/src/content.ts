import type { FillRequest } from './messages';
import { safeHttpOrigin } from './origin';

function formsWithPassword(root: ParentNode): HTMLFormElement[] {
  const forms: HTMLFormElement[] = [];
  const visit = (node: ParentNode) => {
    node.querySelectorAll('form').forEach((form) => {
      if (form.querySelector('input[type="password"]')) forms.push(form);
    });
    node.querySelectorAll('*').forEach((element) => {
      if (element.shadowRoot) visit(element.shadowRoot);
    });
  };
  visit(root);
  return forms;
}

function fill(message: FillRequest) {
  if (safeHttpOrigin(location.href) !== message.origin) return false;
  const forms = formsWithPassword(document);
  const scope: ParentNode | null = forms.length === 1 ? forms[0] : forms.length === 0 ? document : null;
  if (!scope) return false;
  const password = scope.querySelector<HTMLInputElement>('input[type="password"]');
  const username = scope.querySelector<HTMLInputElement>(
    'input[type="email"], input[autocomplete="username"], input[name*="user" i]',
  );
  if (username && message.username) setNativeValue(username, message.username);
  if (password && message.password) setNativeValue(password, message.password);
  return Boolean(password);
}

function setNativeValue(element: HTMLInputElement, value: string) {
  const prototype = Object.getPrototypeOf(element) as object;
  const descriptor = Object.getOwnPropertyDescriptor(prototype, 'value');
  if (descriptor?.set) descriptor.set.call(element, value);
  else element.value = value;
  element.dispatchEvent(new Event('input', { bubbles: true }));
  element.dispatchEvent(new Event('change', { bubbles: true }));
}

function isFillRequest(value: unknown): value is FillRequest {
  return Boolean(value && typeof value === 'object' && 'type' in value && value.type === 'fill');
}

chrome.runtime.onMessage.addListener((message: unknown, _sender, sendResponse) => {
  if (isFillRequest(message)) sendResponse({ ok: fill(message) });
});

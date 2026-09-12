/** Preserve local deep links without allowing a sign-in response to redirect off-site. */
export function safeReturnTo(value: unknown) {
  return typeof value === 'string' &&
    /^\/(apps(?:\/|\?|$)|settings(?:\?|$)|tests(?:\?|$)|runs(?:\?|$))/.test(value) &&
    !value.includes('\\')
    ? value
    : '/apps'
}

/** Move both the viewport and keyboard focus after an explicit user action. */
export function focusSection(
  id: string,
  block: ScrollLogicalPosition = "nearest",
) {
  requestAnimationFrame(() => {
    const target = document.getElementById(id);
    if (!target) return;
    target.focus({ preventScroll: true });
    target.scrollIntoView({
      block,
      behavior: "instant",
    });
  });
}

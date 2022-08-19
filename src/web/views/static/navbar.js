"use strict";

export function init() {
    const $navbarBurgers = Array.prototype.slice.call(document.querySelectorAll('.navbar-burger'), 0);

    $navbarBurgers.forEach(element => {
        element.addEventListener('click', () => {
            const $target = document.getElementById(element.dataset.target);
            element.classList.toggle('is-active');
            $target.classList.toggle('is-active');
        });
    });
}

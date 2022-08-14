"use strict";

export function init() {
    const $navbarBurgers = Array.prototype.slice.call(document.querySelectorAll('.navbar-burger'), 0);

    $navbarBurgers.forEach(element => {
        element.addEventListener('click', () => {
            const target = element.dataset.target;
            const $target = document.getElementById(target);

            element.classList.toggle('is-active');
            $target.classList.toggle('is-active');
        });
    });
}

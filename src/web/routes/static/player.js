"use strict";

(function () {
    function sortVehicles(thSelector) {
        if ((vehicles == null) || (!location.hash.startsWith("#by-"))) {
            return;
        }

        const tbody = vehicles.querySelector("tbody");
        let rows = Array.from(tbody.querySelectorAll("tr"));
        let qs = `[data-sort="${thSelector}"]`;
        rows
            .sort((row_1, row_2) => {
                return parseFloat(row_2.querySelector(qs).dataset.value) - parseFloat(row_1.querySelector(qs).dataset.value);
            })
            .forEach(row => tbody.appendChild(row));

        const iconText = vehicles.querySelector(`${thSelector} span.icon-text`);
        iconText.insertBefore(sortIcon, iconText.firstChild);
    }

    function createSortIcon() {
        const inner = document.createElement("i");
        inner.classList.add("fas", "fa-angle-down");
        const outer = document.createElement("span");
        outer.classList.add("icon");
        outer.appendChild(inner);
        return outer;
    }

    window.onhashchange = function () {
        sortVehicles(location.hash);
    };

    const vehicles = document.getElementById("vehicles");
    const sortIcon = createSortIcon();

    sortVehicles(!!location.hash ? location.hash : "#by-battles");
})();

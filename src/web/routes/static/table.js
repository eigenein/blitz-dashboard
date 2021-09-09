"use strict";

(function () {
    function sortTable(table, by) {
        const tbody = table.querySelector("tbody");
        const qs = `[data-sort="${by}"]`;

        Array
            .from(tbody.querySelectorAll("tr"))
            .sort((row1, row2) => {
                try {
                    return parseFloat(row2.querySelector(qs).dataset.value) - parseFloat(row1.querySelector(qs).dataset.value);
                } catch (error) {
                    console.warn(error);
                    return 0;
                }
            })
            .forEach(row => tbody.appendChild(row));

        table.querySelector(`thead th a${qs} .icon-text`).appendChild(sortIcon);
    }

    function addSortableTableEventListeners(table) {
        table.querySelectorAll("th a").forEach((a) => {
            a.addEventListener("click", () => {
                const sortBy = a.dataset.sort;
                sortTable(table, sortBy);
                localStorage.setItem(`${table.id}SortBy`, sortBy);
            });
        });
    }

    function createSortIcon() {
        const inner = document.createElement("i");
        inner.classList.add("fas", "fa-angle-down");
        const outer = document.createElement("span");
        outer.classList.add("icon");
        outer.appendChild(inner);
        return outer;
    }

    function initSortableTable(table, defaultSortBy) {
        addSortableTableEventListeners(table);
        sortTable(table, localStorage.getItem(`${table.id}SortBy`) || defaultSortBy);
    }

    const vehicles = document.getElementById("vehicles");
    const sortIcon = createSortIcon();

    if (vehicles != null) {
        initSortableTable(vehicles, "battles");
    }
})();

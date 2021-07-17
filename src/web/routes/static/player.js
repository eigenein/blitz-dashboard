(function () {
    function sortVehicles(by) {
        if (!location.hash.startsWith("#by-")) {
            return;
        }
        let qs = `[data-sort="${by}"]`;
        rows.sort((row_1, row_2) => {
            return parseFloat(row_2.querySelector(qs).dataset.value) - parseFloat(row_1.querySelector(qs).dataset.value);
        });
        rows.forEach(row => vehicles.appendChild(row));
        // TODO: set the sort icon.
    }

    window.onhashchange = function () {
        sortVehicles(location.hash);
    };

    const vehicles = document.getElementById("vehicles");
    let rows = null;
    if (vehicles != null) {
        rows = Array.from(vehicles.querySelectorAll("tbody tr"));
        sortVehicles(!!location.hash ? location.hash : "#by-battles");
    }
})();

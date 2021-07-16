(function () {
    function compareValues(left, right) {
        return (left < right) ? +1 : (left > right) ? -1 : 0;
    }

    function sortVehicles(by) {
        console.log(`Sorting vehicles ${by}`);
        let qs = `[data-sort="${by}"]`;
        rows.sort((row_1, row_2) => {
            return parseFloat(row_2.querySelector(qs).dataset.value) - parseFloat(row_1.querySelector(qs).dataset.value);
        });
        rows.forEach(row => vehicles.appendChild(row));
    }

    window.onhashchange = function () {
        sortVehicles(location.hash);
    };

    const vehicles = document.getElementById("vehicles");
    let rows = Array.from(vehicles.querySelectorAll("tr")).slice(1);
    sortVehicles(!!location.hash ? location.hash : "#by-battles");
})();

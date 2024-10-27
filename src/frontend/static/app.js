const board = document.getElementById('board');
const ws = new WebSocket('ws://localhost:3000/ws');

function createBoard() {
    for (let i = 0; i < 64; i++) {
        const cell = document.createElement('div');
        cell.className = 'cell';
        cell.dataset.index = i;
        cell.addEventListener('click', () => makeMove(i));
        board.appendChild(cell);
    }
}

function updateBoard(gameState) {
    const cells = document.querySelectorAll('.cell');
    cells.forEach((cell, index) => {
        cell.innerHTML = '';
        cell.classList.remove('valid-move');
        if (gameState.black.includes(index)) {
            const piece = document.createElement('div');
            piece.className = 'piece black';
            cell.appendChild(piece);
        } else if (gameState.white.includes(index)) {
            const piece = document.createElement('div');
            piece.className = 'piece white';
            cell.appendChild(piece);
        }
        if (gameState.moves.includes(index)) {
            cell.classList.add('valid-move');
        }
    });
}

function makeMove(index) {
    ws.send(JSON.stringify({
        do_move: index
    }));
}

function undoMove() {
    ws.send(JSON.stringify({
        "undo": null
    }));
}

ws.onmessage = (event) => {
    const gameState = JSON.parse(event.data);
    updateBoard(gameState);
};

createBoard();

// Add right-click event listener to the board
board.addEventListener('contextmenu', (e) => {
    e.preventDefault(); // Prevent the default context menu
    undoMove();
});

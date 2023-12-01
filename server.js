const games = {};

const createGame = (playerId) => {
  const newGameId = `game_${Date.now()}`;
  games[newGameId] = {
    players: [playerId, null],
    currentPlayer: 1,
    scores: [0, 0],
    dice: [
      [4, 6, 8, 10, 20].map(rollDie), // TODO: Roll dice according to character sheet
      [],
    ],
  };
  return newGameId;
};

const rollDie = size => ({
  size: size,
  value: Math.ceil(Math.random() * size),
});

const performAttack = (gameId, playerId, attackerDieIndices, defenderDieIndex) => {
  console.log('performAttack', gameId, playerId, attackerDieIndices, defenderDieIndex);
  const game = games[gameId];
  if (!game) {
    return { message: 'Game not found' };
  }

  const currentPlayerIndex = game.players.indexOf(playerId);
  if (game.currentPlayer !== currentPlayerIndex + 1) {
    return { message: 'It is not your turn' };
  }

  const attackerDice = game.dice[(game.currentPlayer - 1) % 2];
  const defenderDice = game.dice[game.currentPlayer % 2];
  console.log('defenderDice', defenderDice);

  // Perform power attack or skill attack based on the number of attacker dice indices
  let attackSuccess = false;
  if (attackerDieIndices.length === 1) {
    // Power attack
    const attackerDie = attackerDice[attackerDieIndices[0]];
    if (attackerDie.value >= defenderDice[defenderDieIndex].value) {
      console.log('Power attack successful', attackerDie, defenderDice[defenderDieIndex]);
      attackSuccess = true;
      game.scores[currentPlayerIndex] += defenderDice[defenderDieIndex].size;
      // Capture the die
      defenderDice.splice(defenderDieIndex, 1);
      // Re-roll attacker die
      attackerDice[attackerDieIndices[0]] = rollDie(attackerDie.size);
    } else {
      // TODO: Fail attack
    }
  } else {
    // Skill attack
    const attackValue = attackerDieIndices.reduce((acc, index) => acc + attackerDice[index].value, 0);
    if (attackValue === defenderDice[defenderDieIndex].value) {
      console.log('Skill attack successful', attackValue, defenderDice[defenderDieIndex]);
      attackSuccess = true;
      game.scores[currentPlayerIndex] += defenderDice[defenderDieIndex].size;
      // Capture the die
      defenderDice.splice(defenderDieIndex, 1);
      // Re-roll attacker dice
      attackerDieIndices.forEach(index => {
        attackerDice[index] = rollDie(attackerDice[index].size);
      });
    } else {
      // TODO: Fail attack
    }
  }

  // Check win condition
  if (defenderDice.length === 0) {
    return { message: `${currentPlayer} wins with a score of ${game.scores[currentPlayerIndex]}` };
  }

  // If attack successful, switch players
  if (attackSuccess) {
    game.currentPlayer = game.currentPlayer % 2 + 1;
  }

  return { message: attackSuccess ? 'Attack successful' : 'Attack failed', game };
};

const getGameStatus = gameId => {
  const game = games[gameId];
  if (!game) {
    return { message: 'Game not found' };
  }

  return game;
}

function getOpenGames() {
  return Object.entries(games)
    .filter(([gameId, game]) => game.players.includes(null))
    .map(([gameId, game]) => ({ gameId, ...game }));
}

function joinGame(gameId, playerId) {
  // Check if the game exists
  const game = games[gameId];
  if (!game) {
    throw new Error(`Game not found: ${gameId}`);
  }

  // Check if the player has already joined
  if (game.players.includes(playerId)) {
    throw new Error(`Player ${playerId} has already joined game ${gameId}`);
  }

  // Find an empty slot for the player
  const playerIndex = game.players.indexOf(null);
  if (playerIndex === -1) {
    throw new Error(`Game is full: ${gameId}`);
  }

  // Assign the player to the game
  game.players[playerIndex] = playerId;
  // TODO: Roll dice according to character sheet
  game.dice[playerIndex] = [4, 6, 8, 10, 20].map(rollDie);

  // Update the game state
  games[gameId] = game;

  return { message: `Player ${playerId} joined game ${gameId} as Player ${playerIndex + 1}` };
}

Bun.serve({
  async fetch(request) {
    const url = new URL(request.url);

    if (url.pathname.startsWith('/api/games')) {
      const [,,, gameId, action] = url.pathname.split('/');

      if (request.method === 'POST') {
        const playerId = request.headers.get('X-Player-Id');
        
        switch (action) {
          case undefined:
            const newGameId = createGame(playerId);
            return new Response(JSON.stringify({ gameId: newGameId }), { status: 200 });
          case 'join':
            return new Response(JSON.stringify(joinGame(gameId, playerId)), { status: 200 });
          case 'attack':
            const requestBody = await request.json();
            const { attackerDieIndices, defenderDieIndex } = requestBody;
            return new Response(JSON.stringify(performAttack(gameId, playerId, attackerDieIndices, defenderDieIndex)), { status: 200 });
          default:
            return new Response('Action not found', { status: 404 });
        }
      } else if (request.method === 'GET') {
        if (action === 'status') {
          return new Response(JSON.stringify(getGameStatus(gameId)), { status: 200 });
        }

        if (!action) {
          return new Response(JSON.stringify(getOpenGames(games)), { status: 200 });
        }
      } else {
        return new Response('Method not allowed', { status: 405 });
      }
    }

    // Return the static files
    try {
      const fileUrl = new URL(url.pathname
        .replace(/^\//, './')
        .replace(/\/$/, '/index.html'),
        new URL('./dist/', new URL(import.meta.url)));
      const file = await Bun.file(fileUrl);
      return new Response(file);
    } catch (e) {
      console.error('Error', e);
      return new Response('Not found', { status: 404 });
    }
  }
});